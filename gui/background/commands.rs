use crossbeam::channel::Sender;
use egui::Context;
use fotobuch::commands::build::{BuildConfig, build};
use fotobuch::commands::config::config_set;
use fotobuch::commands::page::{
    DstMove, DstSwap, PageMoveCmd, PagesExpr, SlotExpr, Src, execute_move,
};
use fotobuch::commands::place::{PlaceConfig, place};
use fotobuch::commands::rebuild::{RebuildScope, rebuild};
use fotobuch::commands::undo::{redo, undo};
use fotobuch::commands::unplace::execute_unplace;

use crate::task::BackgroundResult;

use super::render::send_command_done;

pub(super) fn run_load_photo_thumbnails(
    items: Vec<(String, std::path::PathBuf)>,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    repaint_ctx: &Context,
) {
    for (id, source) in items {
        let tx = result_tx.clone();
        let ctx = repaint_ctx.clone();
        pool.spawn(
            move || match crate::thumbnail::load(&source, super::POOL_THUMB_MAX_EDGE_PX) {
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
            },
        );
    }
}

pub(super) fn run_page_command(cmd: PageMoveCmd, rctx: &mut super::RenderCtx<'_>) {
    let move_output = match execute_move(rctx.project_root, cmd) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
            return;
        }
        Ok(o) => o,
    };

    if move_output.changed_state.is_none() {
        send_command_done(None, vec![], rctx);
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
    match build(rctx.project_root, &build_cfg) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(build_output) => {
            let new_state = build_output.changed_state.or(move_output.changed_state);
            let mut dirty = move_dirty;
            for p in build_output.result.pages_rebuilt {
                if !dirty.contains(&p) {
                    dirty.push(p);
                }
            }
            send_command_done(new_state, dirty, rctx);
        }
    }
}

pub(super) fn run_page_swap(left: usize, right: usize, rctx: &mut super::RenderCtx<'_>) {
    let cmd = PageMoveCmd::Swap {
        left: Src::Pages(PagesExpr::single(left as u32)),
        right: DstSwap::Pages(PagesExpr::single(right as u32)),
    };
    run_page_command(cmd, rctx);
}

pub(super) fn run_swap_slots(
    src_page: usize,
    src_slot: usize,
    dst_page: usize,
    dst_slot: usize,
    rctx: &mut super::RenderCtx<'_>,
) {
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
    run_page_command(cmd, rctx);
}

pub(super) fn run_move_slot(
    src_page: usize,
    src_slots: Vec<usize>,
    dst_page: usize,
    rctx: &mut super::RenderCtx<'_>,
) {
    let cmd = PageMoveCmd::Move {
        src: Src::Slots {
            page: src_page as u32,
            slots: SlotExpr::from_list(src_slots.iter().map(|&s| s as u32).collect()),
        },
        dst: DstMove::Page(dst_page as u32),
    };
    run_page_command(cmd, rctx);
}

pub(super) fn run_undo(rctx: &mut super::RenderCtx<'_>) {
    run_undo_or_redo(undo(rctx.project_root, 1), rctx);
}

pub(super) fn run_redo(rctx: &mut super::RenderCtx<'_>) {
    run_undo_or_redo(redo(rctx.project_root, 1), rctx);
}

fn run_undo_or_redo(
    result: anyhow::Result<fotobuch::commands::CommandOutput<fotobuch::commands::UndoResult>>,
    rctx: &mut super::RenderCtx<'_>,
) {
    match result {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(output) => {
            let new_state = output.changed_state;
            let num_pages = new_state.as_ref().map_or(0, |s| s.layout.len());
            let all_pages: Vec<usize> = (0..num_pages).collect();
            send_command_done(new_state, all_pages, rctx);
        }
    }
}

pub(super) fn run_place_photos(
    photo_ids: Vec<String>,
    dst_page: Option<usize>,
    rctx: &mut super::RenderCtx<'_>,
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
        Ok(output) => {
            send_command_done(output.changed_state, output.result.pages_affected, rctx);
        }
    }
}

pub(super) fn run_config_set(key: &str, value: &str, rctx: &mut super::RenderCtx<'_>) {
    match config_set(rctx.project_root, key, value) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(output) => {
            let num_pages = output
                .changed_state
                .as_ref()
                .map(|s| s.layout.len())
                .unwrap_or(0);
            let all_pages: Vec<usize> = (0..num_pages).collect();
            send_command_done(output.changed_state, all_pages, rctx);
        }
    }
}

pub(super) fn run_move_to_new_page(
    src_page: usize,
    src_slots: Vec<usize>,
    at_position: usize,
    rctx: &mut super::RenderCtx<'_>,
) {
    let cmd = PageMoveCmd::Move {
        src: Src::Slots {
            page: src_page as u32,
            slots: SlotExpr::from_list(src_slots.iter().map(|&s| s as u32).collect()),
        },
        dst: DstMove::NewPageAt(at_position as u32),
    };
    run_page_command(cmd, rctx);
}

pub(super) fn run_swap_range(
    src_page: usize,
    src_slots: Vec<usize>,
    dst_page: usize,
    dst_slots: Vec<usize>,
    rctx: &mut super::RenderCtx<'_>,
) {
    let (s0, s1) = (*src_slots.first().unwrap(), *src_slots.last().unwrap());
    let (d0, d1) = (*dst_slots.first().unwrap(), *dst_slots.last().unwrap());
    let cmd = PageMoveCmd::Swap {
        left: Src::Slots {
            page: src_page as u32,
            slots: if s0 == s1 {
                SlotExpr::single(s0 as u32)
            } else {
                SlotExpr::from_range(s0 as u32, s1 as u32)
            },
        },
        right: DstSwap::Slots {
            page: dst_page as u32,
            slots: if d0 == d1 {
                SlotExpr::single(d0 as u32)
            } else {
                SlotExpr::from_range(d0 as u32, d1 as u32)
            },
        },
    };
    run_page_command(cmd, rctx);
}

pub(super) fn run_unplace(page: usize, slots: Vec<usize>, rctx: &mut super::RenderCtx<'_>) {
    let slot_expr = SlotExpr::from_list(slots.iter().map(|&s| s as u32).collect());
    match execute_unplace(rctx.project_root, page as u32, slot_expr) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(out) => {
            if out.changed_state.is_none() {
                send_command_done(None, vec![], rctx);
                return;
            }
            let build_cfg = BuildConfig {
                release: false,
                force: false,
                pages: None,
            };
            match build(rctx.project_root, &build_cfg) {
                Err(e) => {
                    let _ = rctx
                        .result_tx
                        .send(BackgroundResult::CommandFailed(e.to_string()));
                }
                Ok(build_out) => {
                    let new_state = build_out.changed_state.or(out.changed_state);
                    let mut dirty: Vec<usize> = out
                        .result
                        .pages_modified
                        .iter()
                        .map(|&p| p as usize)
                        .collect();
                    for p in build_out.result.pages_rebuilt {
                        if !dirty.contains(&p) {
                            dirty.push(p);
                        }
                    }
                    send_command_done(new_state, dirty, rctx);
                }
            }
        }
    }
}

pub(super) fn run_delete_pages(pages: Vec<usize>, rctx: &mut super::RenderCtx<'_>) {
    let cmd = PageMoveCmd::Move {
        src: Src::Pages(PagesExpr::from_list(
            pages.iter().map(|&p| p as u32).collect(),
        )),
        dst: DstMove::Unplace,
    };
    run_page_command(cmd, rctx);
}

pub(super) fn run_rebuild_pages(pages: Vec<usize>, rctx: &mut super::RenderCtx<'_>) {
    let mut dirty: Vec<usize> = vec![];
    let mut new_state: Option<Box<fotobuch::dto_models::ProjectState>> = None;
    for p in pages {
        match rebuild(rctx.project_root, RebuildScope::SinglePage(p)) {
            Err(e) => {
                let _ = rctx.result_tx.send(BackgroundResult::CommandFailed(format!(
                    "rebuild page {p}: {e}"
                )));
            }
            Ok(out) => {
                if !dirty.contains(&p) {
                    dirty.push(p);
                }
                if let Some(s) = out.changed_state {
                    new_state = Some(Box::new(s));
                }
            }
        }
    }
    send_command_done(new_state.map(|b| *b), dirty, rctx);
}

pub(super) fn run_rebuild_all(rctx: &mut super::RenderCtx<'_>) {
    match rebuild(rctx.project_root, RebuildScope::All) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(out) => send_command_done(out.changed_state, out.result.pages_rebuilt, rctx),
    }
}

pub(super) fn run_release_build(rctx: &mut super::RenderCtx<'_>) {
    let cfg = BuildConfig {
        release: true,
        force: false,
        pages: None,
    };
    match build(rctx.project_root, &cfg) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(out) => send_command_done(out.changed_state, out.result.pages_rebuilt, rctx),
    }
}
