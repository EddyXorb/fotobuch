mod commands;
mod render;

use std::path::PathBuf;

use crossbeam::channel::{Receiver, Sender, unbounded};
use egui::Context;
use fotobuch::output::typst_world::TypstWorld;

use crate::task::{BackgroundResult, BackgroundTask};

pub(super) const NAV_THUMB_MAX_EDGE_PX: u32 = 512;
pub(super) const POOL_THUMB_MAX_EDGE_PX: u32 = 256;

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

        while let Ok(task) = task_rx.recv() {
            match task {
                BackgroundTask::RenderPages {
                    pages,
                    pixel_per_pt,
                } => {
                    render::render_pages(
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
                    commands::run_swap_slots(
                        &project_root,
                        src_page,
                        src_slot,
                        dst_page,
                        dst_slot,
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
                    commands::run_move_slot(
                        &project_root,
                        src_page,
                        src_slots,
                        dst_page,
                        &mut world,
                        &pool,
                        &result_tx,
                        &repaint_ctx,
                        pixel_per_pt,
                    );
                }
                BackgroundTask::Undo { pixel_per_pt } => {
                    commands::run_undo(
                        &project_root,
                        &mut world,
                        &pool,
                        &result_tx,
                        &repaint_ctx,
                        pixel_per_pt,
                    );
                }
                BackgroundTask::Redo { pixel_per_pt } => {
                    commands::run_redo(
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
                    commands::run_page_swap(
                        &project_root,
                        left,
                        right,
                        &mut world,
                        &pool,
                        &result_tx,
                        &repaint_ctx,
                        pixel_per_pt,
                    );
                }
                BackgroundTask::LoadPhotoThumbnails { items } => {
                    commands::run_load_photo_thumbnails(items, &pool, &result_tx, &repaint_ctx);
                }
                BackgroundTask::PlacePhotos {
                    photo_ids,
                    dst_page,
                    pixel_per_pt,
                } => {
                    commands::run_place_photos(
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
                    commands::run_config_set(
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
