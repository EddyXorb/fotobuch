use crossbeam::channel::Sender;
use egui::Context;
use fotobuch::commands::page::SlotExpr;
use fotobuch::commands::place::{PlaceConfig, place};
use fotobuch::commands::unplace::execute_unplace;

use crate::task::BackgroundResult;

pub fn run_load_photo_thumbnails(
    items: Vec<(String, std::path::PathBuf)>,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    repaint_ctx: &Context,
) {
    for (id, source) in items {
        let tx = result_tx.clone();
        let ctx = repaint_ctx.clone();
        pool.spawn(move || {
            match crate::thumbnail::load(&source, crate::background::POOL_THUMB_MAX_EDGE_PX) {
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

pub fn run_place_photos(
    photo_ids: Vec<String>,
    dst_page: Option<usize>,
    rctx: &mut crate::background::RenderCtx<'_>,
) {
    let config = PlaceConfig {
        filters: vec![],
        ids: photo_ids,
        into_page: dst_page,
    };
    match place(rctx.project_root, &config) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(place_out) => {
            let dirty = place_out.result.pages_affected;
            super::page::build_after_command(place_out.changed_state, dirty, rctx);
        }
    }
}

pub fn run_unplace(page: usize, slots: Vec<usize>, rctx: &mut crate::background::RenderCtx<'_>) {
    let slot_expr = SlotExpr::from_list(slots.iter().map(|&s| s as u32).collect());
    match execute_unplace(rctx.project_root, page as u32, slot_expr) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(out) => {
            let dirty: Vec<usize> = out
                .result
                .pages_modified
                .iter()
                .map(|&p| p as usize)
                .collect();
            super::page::build_after_command(out.changed_state, dirty, rctx);
        }
    }
}
