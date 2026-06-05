use crossbeam::channel::Sender;
use egui::Context;
use fotobuch::commands::PlaceDst;
use fotobuch::commands::page::SlotExpr;
use fotobuch::commands::place::{PlaceConfig, place};
use fotobuch::commands::unplace::execute_unplace;

use crate::task::BackgroundResult;

pub fn run_add_photos(
    paths: Vec<std::path::PathBuf>,
    recursive: bool,
    weight: f64,
    source_filter: String,
    rctx: &mut crate::background::RenderCtx<'_>,
) {
    use fotobuch::commands::add::{AddConfig, add};
    use regex::Regex;

    let source_filters = if source_filter.is_empty() {
        vec![]
    } else {
        match Regex::new(&source_filter) {
            Ok(re) => vec![re],
            Err(e) => {
                let _ = rctx.result_tx.send(BackgroundResult::CommandFailed(format!(
                    "invalid filter regex: {e}"
                )));
                return;
            }
        }
    };
    let cfg = AddConfig {
        paths,
        allow_duplicates: false,
        xmp_filters: vec![],
        source_filters,
        dry_run: false,
        update: false,
        recursive,
        weight,
    };
    match add(rctx.project_root, &cfg) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(out) => {
            crate::background::send_command_done(out.changed_state, vec![], rctx);
        }
    }
}

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
    dst: PlaceDst,
    rctx: &mut crate::background::RenderCtx<'_>,
) {
    let config = PlaceConfig {
        filters: vec![],
        ids: photo_ids,
        dst,
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

pub fn run_remove_photos(photo_ids: Vec<String>, rctx: &mut crate::background::RenderCtx<'_>) {
    use fotobuch::commands::remove::{RemoveConfig, RemoveTarget, remove};
    let cfg = RemoveConfig {
        target: RemoveTarget::Ids(photo_ids),
        keep_files: false,
    };
    match remove(rctx.project_root, &cfg) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(out) => {
            let dirty = out.result.pages_affected;
            super::page::build_after_command(out.changed_state, dirty, rctx);
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
            // If all modified pages are now empty, skip the solver build.
            // An empty page has nothing to lay out; running build would trigger
            // an unwanted full rebuild instead of a cheap no-op.
            let all_pages_empty = out.changed_state.as_ref().is_some_and(|s| {
                dirty
                    .iter()
                    .all(|&p| s.layout.get(p).is_none_or(|lp| lp.photos.is_empty()))
            });
            if all_pages_empty {
                crate::background::send_command_done(out.changed_state, dirty, rctx);
            } else {
                super::page::build_after_command(out.changed_state, dirty, rctx);
            }
        }
    }
}
