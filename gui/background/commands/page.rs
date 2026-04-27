use fotobuch::commands::build::{BuildConfig, build};
use fotobuch::commands::page::{
    DstMove, DstSwap, PageMoveCmd, PagesExpr, SlotExpr, Src, execute_move,
};

use super::super::render::send_command_done;
use crate::task::BackgroundResult;

pub fn build_after_command(
    cmd_changed_state: Option<fotobuch::dto_models::ProjectState>,
    cmd_dirty: Vec<usize>,
    rctx: &mut super::super::RenderCtx<'_>,
) {
    if cmd_changed_state.is_none() {
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
            let new_state = build_out.changed_state.or(cmd_changed_state);
            let mut dirty = cmd_dirty;
            for p in build_out.result.pages_rebuilt {
                if !dirty.contains(&p) {
                    dirty.push(p);
                }
            }
            send_command_done(new_state, dirty, rctx);
        }
    }
}

pub fn run_page_command(cmd: PageMoveCmd, rctx: &mut super::super::RenderCtx<'_>) {
    let move_output = match execute_move(rctx.project_root, cmd) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
            return;
        }
        Ok(o) => o,
    };
    let dirty: Vec<usize> = move_output
        .result
        .pages_modified
        .iter()
        .chain(move_output.result.pages_inserted.iter())
        .map(|&p| p as usize)
        .collect();
    build_after_command(move_output.changed_state, dirty, rctx);
}

pub fn run_page_swap(left: usize, right: usize, rctx: &mut super::super::RenderCtx<'_>) {
    let cmd = PageMoveCmd::Swap {
        left: Src::Pages(PagesExpr::single(left as u32)),
        right: DstSwap::Pages(PagesExpr::single(right as u32)),
    };
    run_page_command(cmd, rctx);
}

pub fn run_swap_slots(
    src_page: usize,
    src_slot: usize,
    dst_page: usize,
    dst_slot: usize,
    rctx: &mut super::super::RenderCtx<'_>,
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

pub fn run_move_slot(
    src_page: usize,
    src_slots: Vec<usize>,
    dst_page: usize,
    rctx: &mut super::super::RenderCtx<'_>,
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

pub fn run_move_to_new_page(
    src_page: usize,
    src_slots: Vec<usize>,
    at_position: usize,
    rctx: &mut super::super::RenderCtx<'_>,
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

pub fn run_swap_range(
    src_page: usize,
    src_slots: Vec<usize>,
    dst_page: usize,
    dst_slots: Vec<usize>,
    rctx: &mut super::super::RenderCtx<'_>,
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

pub fn run_delete_pages(pages: Vec<usize>, rctx: &mut super::super::RenderCtx<'_>) {
    let cmd = PageMoveCmd::Move {
        src: Src::Pages(PagesExpr::from_list(
            pages.iter().map(|&p| p as u32).collect(),
        )),
        dst: DstMove::Unplace,
    };
    run_page_command(cmd, rctx);
}

pub fn run_move_page(src_page: usize, at_position: usize, rctx: &mut super::super::RenderCtx<'_>) {
    let cmd = PageMoveCmd::Move {
        src: Src::Pages(PagesExpr::single(src_page as u32)),
        dst: DstMove::NewPageAt(at_position as u32),
    };
    run_page_command(cmd, rctx);
}
