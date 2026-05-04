mod commands;
mod render;

use std::path::PathBuf;

use crossbeam::channel::{Receiver, Sender, unbounded};
use egui::Context;
use fotobuch::output::typst_world::TypstWorld;

use crate::task::{BackgroundResult, BackgroundTask};

pub(super) const NAV_THUMB_MAX_EDGE_PX: u32 = 512;
pub(crate) const POOL_THUMB_MAX_EDGE_PX: u32 = 256;

pub(crate) use render::send_command_done;

pub(crate) struct RenderCtx<'a> {
    pub project_root: &'a std::path::Path,
    pub pool: &'a rayon::ThreadPool,
    pub result_tx: &'a Sender<BackgroundResult>,
    pub ctx: &'a Context,
    pub world: &'a mut TypstWorld,
    pub pixel_per_pt: f32,
}

pub fn spawn(
    project_root: PathBuf,
    project_name: String,
    repaint_ctx: Context,
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

        let mut rctx = RenderCtx {
            project_root: &project_root,
            pool: &pool,
            result_tx: &result_tx,
            ctx: &repaint_ctx,
            world: &mut world,
            pixel_per_pt: 1.0,
        };

        while let Ok(task) = task_rx.recv() {
            match task {
                BackgroundTask::RenderPages {
                    pages,
                    pixel_per_pt,
                } => {
                    rctx.pixel_per_pt = pixel_per_pt;
                    render::render_pages(&mut rctx, pages);
                }
                BackgroundTask::SetPixelPerPt(ppt) => {
                    rctx.pixel_per_pt = ppt;
                }
                BackgroundTask::Swap {
                    src_page,
                    src_slot,
                    dst_page,
                    dst_slot,
                } => {
                    commands::run_swap_slots(src_page, src_slot, dst_page, dst_slot, &mut rctx);
                }
                BackgroundTask::Move {
                    src_page,
                    src_slots,
                    dst_page,
                } => {
                    commands::run_move_slot(src_page, src_slots, dst_page, &mut rctx);
                }
                BackgroundTask::Undo => {
                    commands::run_undo(&mut rctx);
                }
                BackgroundTask::Redo => {
                    commands::run_redo(&mut rctx);
                }
                BackgroundTask::PageSwap { left, right } => {
                    commands::run_page_swap(left, right, &mut rctx);
                }
                BackgroundTask::LoadPhotoThumbnails { items } => {
                    commands::run_load_photo_thumbnails(items, &pool, &result_tx, &repaint_ctx);
                }
                BackgroundTask::Place {
                    photo_ids,
                    dst_page,
                } => {
                    commands::run_place_photos(photo_ids, dst_page, &mut rctx);
                }
                BackgroundTask::ConfigSet { key, value } => {
                    commands::run_config_set(&key, &value, &mut rctx);
                }
                BackgroundTask::MoveToNewPage {
                    src_page,
                    src_slots,
                    at_position,
                } => {
                    commands::run_move_to_new_page(src_page, src_slots, at_position, &mut rctx);
                }
                BackgroundTask::SwapRange {
                    src_page,
                    src_slots,
                    dst_page,
                    dst_slots,
                } => {
                    commands::run_swap_range(src_page, src_slots, dst_page, dst_slots, &mut rctx);
                }
                BackgroundTask::Unplace { page, slots } => {
                    commands::run_unplace(page, slots, &mut rctx);
                }
                BackgroundTask::DeletePages { pages } => {
                    commands::run_delete_pages(pages, &mut rctx);
                }
                BackgroundTask::MovePage {
                    src_page,
                    at_position,
                } => {
                    commands::run_move_page(src_page, at_position, &mut rctx);
                }
                BackgroundTask::RebuildPages { pages } => {
                    commands::run_rebuild_pages(pages, &mut rctx);
                }
                BackgroundTask::RebuildAll => {
                    commands::run_rebuild_all(&mut rctx);
                }
                BackgroundTask::ReleaseBuild => {
                    commands::run_release_build(&mut rctx);
                }
            }
        }
    });

    (task_tx, result_rx)
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

        let timeout = Duration::from_secs(30);
        let mut page_rendered = false;
        let mut total_page_count: Option<usize> = None;
        for _ in 0..2 {
            match result_rx
                .recv_timeout(timeout)
                .expect("no result within timeout")
            {
                BackgroundResult::TotalPageCount(n) => total_page_count = Some(n),
                BackgroundResult::PageRendered {
                    page: r,
                    thumb,
                    rasterize_duration,
                    compile_duration,
                } => {
                    assert_eq!(r.page, 0);
                    assert!(!r.pixels.is_empty());
                    assert_eq!(thumb.page, 0);
                    assert!(thumb.width <= 512 && thumb.height <= 512);
                    assert!(rasterize_duration.as_secs() < 30);
                    assert!(compile_duration.as_secs() < 30);
                    page_rendered = true;
                }
                BackgroundResult::Error(e) => panic!("worker error: {e}"),
                other => panic!("unexpected result: {other:?}"),
            }
        }
        assert!(page_rendered, "no PageRendered received");
        assert_eq!(total_page_count, Some(1), "expected TotalPageCount(1)");

        match result_rx.recv_timeout(Duration::from_millis(200)) {
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {}
            Ok(_) => panic!("unexpected third result"),
        }
    }
}
