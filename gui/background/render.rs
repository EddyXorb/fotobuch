use std::sync::Arc;
use std::time::Instant;

use fotobuch::output::render::{downsample, rasterize_page};

use crate::task::BackgroundResult;

pub(super) fn send_command_done(
    new_state: Option<fotobuch::dto_models::ProjectState>,
    dirty_pages: Vec<usize>,
    rctx: &mut super::RenderCtx<'_>,
) {
    let has_change = new_state.is_some();
    let _ = rctx.result_tx.send(BackgroundResult::CommandDone {
        new_state: new_state.map(Box::new),
        dirty_pages: dirty_pages.clone(),
    });
    if has_change {
        render_pages(rctx, dirty_pages);
    }
    rctx.ctx.request_repaint();
}

pub(super) fn render_pages(rctx: &mut super::RenderCtx<'_>, pages: Vec<usize>) {
    if pages.is_empty() {
        return;
    }
    if let Err(e) = rctx.world.reload() {
        let _ = rctx.result_tx.send(BackgroundResult::Error(e.to_string()));
        return;
    }

    let t_compile = Instant::now();
    let doc = match rctx.world.compile_document() {
        Ok(d) => d,
        Err(e) => {
            let _ = rctx.result_tx.send(BackgroundResult::Error(e.to_string()));
            return;
        }
    };
    let compile_duration = t_compile.elapsed();
    let doc_page_count = doc.pages.len();

    let _ = rctx
        .result_tx
        .send(BackgroundResult::TotalPageCount(doc_page_count));

    let max_requested = pages.iter().copied().max().unwrap_or(0);
    let extra_pages = (max_requested + 1)..doc_page_count;
    let all_pages: Vec<usize> = pages.into_iter().chain(extra_pages).collect();

    let doc = Arc::new(doc);
    for page in all_pages {
        let doc = Arc::clone(&doc);
        let tx = rctx.result_tx.clone();
        let ctx = rctx.ctx.clone();
        let pixel_per_pt = rctx.pixel_per_pt;
        rctx.pool.spawn(move || {
            let t_raster = Instant::now();
            match rasterize_page(&doc, page, pixel_per_pt) {
                Ok(rendered) => {
                    let thumb = downsample(&rendered, super::NAV_THUMB_MAX_EDGE_PX);
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
