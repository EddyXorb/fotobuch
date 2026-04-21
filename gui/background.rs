use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crossbeam::channel::{Receiver, Sender, unbounded};
use egui::Context;
use fotobuch::commands::build::{BuildConfig, build};
use fotobuch::commands::config::config_set;
use fotobuch::commands::page::{DstMove, DstSwap, PageMoveCmd, SlotExpr, Src, execute_move};
use fotobuch::commands::place::{PlaceConfig, place};
use fotobuch::commands::undo::{redo, undo};
use fotobuch::dto_models::ProjectState;
use fotobuch::output::render::{downsample, rasterize_page};
use fotobuch::output::typst_world::TypstWorld;

use crate::task::{BackgroundResult, BackgroundTask};

const NAV_THUMB_MAX_EDGE_PX: u32 = 120;
const POOL_THUMB_MAX_EDGE_PX: u32 = 256;

pub fn spawn(
    project_root: PathBuf,
    project_name: String,
    repaint_ctx: egui::Context,
) -> (Sender<BackgroundTask>, Receiver<BackgroundResult>) {
    let (task_tx, task_rx) = unbounded::<BackgroundTask>();
    let (result_tx, result_rx) = unbounded::<BackgroundResult>();

    std::thread::spawn(move || {
        let pool = build_pool();

        let mut world = match TypstWorld::new(&project_root, &project_name) {
            Ok(w) => w,
            Err(e) => {
                let _ = result_tx.send(BackgroundResult::Error(e.to_string()));
                return;
            }
        };

        while let Ok(task) = task_rx.recv() {
            match task {
                BackgroundTask::RenderPages {
                    pages,
                    pixel_per_pt,
                } => {
                    render_pages(
                        &mut world,
                        &pool,
                        &result_tx,
                        pages,
                        pixel_per_pt,
                        &repaint_ctx,
                    );
                }

                BackgroundTask::SwapSlots {
                    src_page,
                    src_slot,
                    dst_page,
                    dst_slot,
                    pixel_per_pt,
                } => {
                    let cmd = PageMoveCmd::Swap {
                        left: Src::Slots {
                            page: src_page as u32,
                            slots: SlotExpr::single(src_slot as u32),
                        },
                        right: DstSwap::Slots {
                            page: dst_page as u32,
                            slots: SlotExpr::single(dst_slot as u32),
                        },
                    };
                    run_page_command(
                        &project_root,
                        cmd,
                        &mut world,
                        &pool,
                        &result_tx,
                        &repaint_ctx,
                        pixel_per_pt,
                    );
                }

                BackgroundTask::MoveSlot {
                    src_page,
                    src_slots,
                    dst_page,
                    pixel_per_pt,
                } => {
                    let cmd = PageMoveCmd::Move {
                        src: Src::Slots {
                            page: src_page as u32,
                            slots: SlotExpr::from_list(
                                src_slots.iter().map(|&s| s as u32).collect(),
                            ),
                        },
                        dst: DstMove::Page(dst_page as u32),
                    };
                    run_page_command(
                        &project_root,
                        cmd,
                        &mut world,
                        &pool,
                        &result_tx,
                        &repaint_ctx,
                        pixel_per_pt,
                    );
                }

                BackgroundTask::Undo { pixel_per_pt } => {
                    run_undo(
                        &project_root,
                        &mut world,
                        &pool,
                        &result_tx,
                        &repaint_ctx,
                        pixel_per_pt,
                    );
                }

                BackgroundTask::Redo { pixel_per_pt } => {
                    run_redo(
                        &project_root,
                        &mut world,
                        &pool,
                        &result_tx,
                        &repaint_ctx,
                        pixel_per_pt,
                    );
                }

                BackgroundTask::PageSwap {
                    left,
                    right,
                    pixel_per_pt,
                } => {
                    use fotobuch::commands::page::{DstSwap, PageMoveCmd, PagesExpr, Src};
                    let cmd = PageMoveCmd::Swap {
                        left: Src::Pages(PagesExpr::single(left as u32)),
                        right: DstSwap::Pages(PagesExpr::single(right as u32)),
                    };
                    run_page_command(
                        &project_root,
                        cmd,
                        &mut world,
                        &pool,
                        &result_tx,
                        &repaint_ctx,
                        pixel_per_pt,
                    );
                }

                BackgroundTask::LoadPhotoThumbnails { items } => {
                    for (id, source) in items {
                        let tx = result_tx.clone();
                        let ctx = repaint_ctx.clone();
                        pool.spawn(move || {
                            match crate::thumbnail::load(&source, POOL_THUMB_MAX_EDGE_PX) {
                                Ok((w, h, pixels)) => {
                                    let _ = tx.send(BackgroundResult::PhotoThumbnailReady {
                                        id,
                                        width: w,
                                        height: h,
                                        pixels,
                                    });
                                    ctx.request_repaint();
                                }
                                Err(e) => {
                                    tracing::warn!(%e, photo=%id, "thumb load failed");
                                }
                            }
                        });
                    }
                }

                BackgroundTask::PlacePhotos {
                    photo_ids,
                    dst_page,
                    pixel_per_pt,
                } => {
                    run_place_photos(
                        &project_root,
                        photo_ids,
                        dst_page,
                        &mut world,
                        &pool,
                        &result_tx,
                        &repaint_ctx,
                        pixel_per_pt,
                    );
                }

                BackgroundTask::ConfigSet {
                    key,
                    value,
                    pixel_per_pt,
                } => {
                    run_config_set(
                        &project_root,
                        &key,
                        &value,
                        &mut world,
                        &pool,
                        &result_tx,
                        &repaint_ctx,
                        pixel_per_pt,
                    );
                }
            }
        }
    });

    (task_tx, result_rx)
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn run_page_command(
    project_root: &std::path::Path,
    cmd: PageMoveCmd,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    let move_output = match execute_move(project_root, cmd) {
        Err(e) => {
            let _ = result_tx.send(BackgroundResult::CommandFailed(e.to_string()));
            return;
        }
        Ok(o) => o,
    };

    if move_output.changed_state.is_none() {
        send_command_done(None, vec![], world, pool, result_tx, ctx, pixel_per_pt);
        return;
    }

    let move_dirty: Vec<usize> = move_output
        .result
        .pages_modified
        .iter()
        .chain(move_output.result.pages_inserted.iter())
        .map(|&p| p as usize)
        .collect();

    let build_cfg = BuildConfig {
        release: false,
        force: false,
        pages: None,
    };
    match build(project_root, &build_cfg) {
        Err(e) => {
            let _ = result_tx.send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(build_output) => {
            let new_state = build_output.changed_state.or(move_output.changed_state);
            let mut dirty = move_dirty;
            for p in build_output.result.pages_rebuilt {
                if !dirty.contains(&p) {
                    dirty.push(p);
                }
            }
            send_command_done(new_state, dirty, world, pool, result_tx, ctx, pixel_per_pt);
        }
    }
}

fn run_undo(
    project_root: &std::path::Path,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    run_undo_or_redo(
        undo(project_root, 1),
        world,
        pool,
        result_tx,
        ctx,
        pixel_per_pt,
    );
}

fn run_redo(
    project_root: &std::path::Path,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    run_undo_or_redo(
        redo(project_root, 1),
        world,
        pool,
        result_tx,
        ctx,
        pixel_per_pt,
    );
}

fn run_undo_or_redo(
    result: anyhow::Result<fotobuch::commands::CommandOutput<fotobuch::commands::UndoResult>>,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    match result {
        Err(e) => {
            let _ = result_tx.send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(output) => {
            let new_state = output.changed_state;
            let num_pages = new_state.as_ref().map_or(0, |s| s.layout.len());
            let all_pages: Vec<usize> = (0..num_pages).collect();
            send_command_done(
                new_state,
                all_pages,
                world,
                pool,
                result_tx,
                ctx,
                pixel_per_pt,
            );
        }
    }
}

/// Sends `CommandDone` and then kicks off re-rendering of the dirty pages.
fn send_command_done(
    new_state: Option<ProjectState>,
    dirty_pages: Vec<usize>,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    let has_change = new_state.is_some();
    let _ = result_tx.send(BackgroundResult::CommandDone {
        new_state: new_state.map(Box::new),
        dirty_pages: dirty_pages.clone(),
    });
    if has_change {
        render_pages(world, pool, result_tx, dirty_pages, pixel_per_pt, ctx);
    }
    ctx.request_repaint();
}

fn render_pages(
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    pages: Vec<usize>,
    pixel_per_pt: f32,
    ctx: &Context,
) {
    if pages.is_empty() {
        return;
    }
    if let Err(e) = world.reload() {
        let _ = result_tx.send(BackgroundResult::Error(e.to_string()));
        return;
    }

    let t_compile = Instant::now();
    let doc = match world.compile_document() {
        Ok(d) => d,
        Err(e) => {
            let _ = result_tx.send(BackgroundResult::Error(e.to_string()));
            return;
        }
    };
    let compile_duration = t_compile.elapsed();

    let doc = Arc::new(doc);
    for page in pages {
        let doc = Arc::clone(&doc);
        let tx = result_tx.clone();
        let ctx = ctx.clone();
        pool.spawn(move || {
            let t_raster = Instant::now();
            match rasterize_page(&doc, page, pixel_per_pt) {
                Ok(rendered) => {
                    let thumb = downsample(&rendered, NAV_THUMB_MAX_EDGE_PX);
                    let _ = tx.send(BackgroundResult::PageRendered {
                        page: rendered,
                        thumb,
                        rasterize_duration: t_raster.elapsed(),
                        compile_duration,
                    });
                    ctx.request_repaint();
                }
                Err(e) => {
                    let _ = tx.send(BackgroundResult::Error(e.to_string()));
                }
            }
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn run_place_photos(
    project_root: &std::path::Path,
    photo_ids: Vec<String>,
    dst_page: Option<usize>,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    let config = PlaceConfig {
        filters: vec![],
        into_page: dst_page,
    };
    match place(project_root, &config) {
        Err(e) => {
            let _ = result_tx.send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(output) => {
            let dirty = output.result.pages_affected;
            let _ = photo_ids; // IDs were used to determine intent; place handles unplaced logic
            send_command_done(
                output.changed_state,
                dirty,
                world,
                pool,
                result_tx,
                ctx,
                pixel_per_pt,
            );
        }
    }
}

fn run_config_set(
    project_root: &std::path::Path,
    key: &str,
    value: &str,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    match config_set(project_root, key, value) {
        Err(e) => {
            let _ = result_tx.send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(output) => {
            // Config change may affect all pages (e.g. page size, margins)
            let num_pages = output
                .changed_state
                .as_ref()
                .map(|s| s.layout.len())
                .unwrap_or(0);
            let all_pages: Vec<usize> = (0..num_pages).collect();
            send_command_done(
                output.changed_state,
                all_pages,
                world,
                pool,
                result_tx,
                ctx,
                pixel_per_pt,
            );
        }
    }
}

fn build_pool() -> rayon::ThreadPool {
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .saturating_sub(1)
        .max(1);
    rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .expect("failed to build render thread pool")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Duration;

    use crossbeam::channel::RecvTimeoutError;
    use tempfile::TempDir;

    use super::*;
    use crate::task::BackgroundResult;

    #[test]
    fn worker_renders_single_page() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("test.typ"),
            "#set page(width: 50mm, height: 50mm)\nHello",
        )
        .unwrap();

        let (task_tx, result_rx) = spawn(
            temp.path().to_owned(),
            "test".to_owned(),
            egui::Context::default(),
        );
        task_tx
            .send(BackgroundTask::RenderPages {
                pages: vec![0],
                pixel_per_pt: 1.0,
            })
            .unwrap();

        let result = result_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("no result within timeout");

        match result {
            BackgroundResult::PageRendered {
                page: r,
                thumb,
                rasterize_duration,
                compile_duration,
            } => {
                assert_eq!(r.page, 0);
                assert!(!r.pixels.is_empty());
                assert_eq!(thumb.page, 0);
                assert!(thumb.width <= 120 && thumb.height <= 120);
                assert!(rasterize_duration.as_secs() < 30);
                assert!(compile_duration.as_secs() < 30);
            }
            BackgroundResult::Error(e) => panic!("worker error: {e}"),
            other => panic!("unexpected result: {other:?}"),
        }

        match result_rx.recv_timeout(Duration::from_millis(100)) {
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {}
            Ok(_) => panic!("unexpected second result"),
        }
    }
}
