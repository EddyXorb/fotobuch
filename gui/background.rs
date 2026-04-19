use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crossbeam::channel::{Receiver, Sender, unbounded};
use fotobuch::commands::build::{BuildConfig, build};
use fotobuch::commands::page::{DstMove, DstSwap, PageMoveCmd, SlotExpr, Src, execute_move};
use fotobuch::commands::undo::{redo, undo};
use fotobuch::dto_models::ProjectState;
use fotobuch::output::render::rasterize_page;
use fotobuch::output::typst_world::TypstWorld;

use crate::task::{BackgroundResult, BackgroundTask};

pub fn spawn(
    project_root: PathBuf,
    project_name: String,
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
                    render_pages(&mut world, &pool, &result_tx, pages, pixel_per_pt);
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
                        pixel_per_pt,
                    );
                }

                BackgroundTask::Undo { pixel_per_pt } => {
                    run_undo(&project_root, &mut world, &pool, &result_tx, pixel_per_pt);
                }

                BackgroundTask::Redo { pixel_per_pt } => {
                    run_redo(&project_root, &mut world, &pool, &result_tx, pixel_per_pt);
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
    pixel_per_pt: f32,
) {
    let move_output = match execute_move(project_root, cmd) {
        Err(e) => {
            let _ = result_tx.send(BackgroundResult::CommandFailed(e.to_string()));
            return;
        }
        Ok(o) => o,
    };

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
            let new_state = build_output
                .changed_state
                .or(move_output.changed_state)
                .unwrap_or_default();
            let mut dirty = move_dirty;
            for p in build_output.result.pages_rebuilt {
                if !dirty.contains(&p) {
                    dirty.push(p);
                }
            }
            send_command_done(new_state, dirty, world, pool, result_tx, pixel_per_pt);
        }
    }
}

fn run_undo(
    project_root: &std::path::Path,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    pixel_per_pt: f32,
) {
    run_undo_or_redo(undo(project_root, 1), world, pool, result_tx, pixel_per_pt);
}

fn run_redo(
    project_root: &std::path::Path,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    pixel_per_pt: f32,
) {
    run_undo_or_redo(redo(project_root, 1), world, pool, result_tx, pixel_per_pt);
}

fn run_undo_or_redo(
    result: anyhow::Result<fotobuch::commands::CommandOutput<fotobuch::commands::UndoResult>>,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    pixel_per_pt: f32,
) {
    match result {
        Err(e) => {
            let _ = result_tx.send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(output) => {
            let new_state = output.changed_state.unwrap_or_default();
            let all_pages: Vec<usize> = (0..new_state.layout.len()).collect();
            send_command_done(new_state, all_pages, world, pool, result_tx, pixel_per_pt);
        }
    }
}

/// Sends `CommandDone` and then kicks off re-rendering of the dirty pages.
fn send_command_done(
    new_state: ProjectState,
    dirty_pages: Vec<usize>,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    pixel_per_pt: f32,
) {
    let _ = result_tx.send(BackgroundResult::CommandDone {
        new_state: Box::new(new_state),
        dirty_pages: dirty_pages.clone(),
    });
    render_pages(world, pool, result_tx, dirty_pages, pixel_per_pt);
}

fn render_pages(
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    pages: Vec<usize>,
    pixel_per_pt: f32,
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
        pool.spawn(move || {
            let t_raster = Instant::now();
            match rasterize_page(&doc, page, pixel_per_pt) {
                Ok(rendered) => {
                    let _ = tx.send(BackgroundResult::PageRendered {
                        page: rendered,
                        rasterize_duration: t_raster.elapsed(),
                        compile_duration,
                    });
                }
                Err(e) => {
                    let _ = tx.send(BackgroundResult::Error(e.to_string()));
                }
            }
        });
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

        let (task_tx, result_rx) = spawn(temp.path().to_owned(), "test".to_owned());
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
                rasterize_duration,
                compile_duration,
            } => {
                assert_eq!(r.page, 0);
                assert!(!r.pixels.is_empty());
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
