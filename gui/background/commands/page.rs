use fotobuch::commands::build::{BuildConfig, build};
use fotobuch::commands::page::{
    DstMove, DstSwap, PageMoveCmd, PagesExpr, SlotExpr, Src, execute_move,
};

use crate::background::send_command_done;
use crate::task::BackgroundResult;

pub fn build_after_command(
    cmd_changed_state: Option<fotobuch::dto_models::ProjectState>,
    cmd_dirty: Vec<usize>,
    rctx: &mut crate::background::RenderCtx<'_>,
) {
    if cmd_changed_state.is_none() && cmd_dirty.is_empty() {
        send_command_done(None, vec![], rctx);
        return;
    }
    let build_cfg = BuildConfig {
        release: false,
        force: false,
        pages: None,
        skip_pdf: false,
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

pub fn run_page_command(cmd: PageMoveCmd, rctx: &mut crate::background::RenderCtx<'_>) {
    let move_output = match execute_move(rctx.project_root, cmd) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
            return;
        }
        Ok(o) => o,
    };
    let dirty: Vec<usize> = if !move_output.result.pages_deleted.is_empty() {
        // When pages are deleted, all remaining pages from the first deleted index onward
        // shift indices and need re-rendering.
        let new_len = move_output
            .changed_state
            .as_ref()
            .map(|s| s.layout.len())
            .unwrap_or(0);
        (0..new_len).collect()
    } else {
        move_output
            .result
            .pages_modified
            .iter()
            .chain(move_output.result.pages_inserted.iter())
            .map(|&p| p as usize)
            .collect()
    };
    build_after_command(move_output.changed_state, dirty, rctx);
}

pub fn run_page_swap(left: usize, right: usize, rctx: &mut crate::background::RenderCtx<'_>) {
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
    rctx: &mut crate::background::RenderCtx<'_>,
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
    rctx: &mut crate::background::RenderCtx<'_>,
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
    rctx: &mut crate::background::RenderCtx<'_>,
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
    rctx: &mut crate::background::RenderCtx<'_>,
) {
    let (Some(s0), Some(s1)) = (src_slots.first(), src_slots.last()) else {
        let _ = rctx
            .result_tx
            .send(crate::task::BackgroundResult::CommandFailed(
                "empty slot list".into(),
            ));
        return;
    };
    let (Some(d0), Some(d1)) = (dst_slots.first(), dst_slots.last()) else {
        let _ = rctx
            .result_tx
            .send(crate::task::BackgroundResult::CommandFailed(
                "empty slot list".into(),
            ));
        return;
    };
    let (s0, s1) = (*s0, *s1);
    let (d0, d1) = (*d0, *d1);
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

pub fn run_delete_pages(pages: Vec<usize>, rctx: &mut crate::background::RenderCtx<'_>) {
    let cmd = PageMoveCmd::Move {
        src: Src::Pages(PagesExpr::from_list(
            pages.iter().map(|&p| p as u32).collect(),
        )),
        dst: DstMove::Unplace,
    };
    run_page_command(cmd, rctx);
}

pub fn run_move_page(
    src_page: usize,
    at_position: usize,
    rctx: &mut crate::background::RenderCtx<'_>,
) {
    let cmd = PageMoveCmd::Move {
        src: Src::Pages(PagesExpr::single(src_page as u32)),
        dst: DstMove::NewPageAt(at_position as u32),
    };
    run_page_command(cmd, rctx);
}

pub fn run_set_page_mode(
    page: usize,
    mode: fotobuch::dto_models::PageMode,
    rctx: &mut crate::background::RenderCtx<'_>,
) {
    use fotobuch::commands::page::{PagesExpr, execute_mode};
    use fotobuch::dto_models::PageMode;
    match execute_mode(rctx.project_root, PagesExpr::single(page as u32), mode) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(crate::task::BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(out) => {
            if mode == PageMode::Auto {
                build_after_command(out.changed_state, vec![page], rctx);
            } else {
                crate::background::send_command_done(out.changed_state, vec![], rctx);
            }
        }
    }
}

pub fn run_page_pos(
    page: usize,
    slot: usize,
    pos_mode: crate::task::PagePosMode,
    scale: Option<f64>,
    rctx: &mut crate::background::RenderCtx<'_>,
) {
    use crate::task::PagePosMode;
    use fotobuch::commands::page::{PosConfig, PosMode, SlotExpr, execute_pos};
    let cfg = PosConfig {
        position: Some(match pos_mode {
            PagePosMode::Relative { dx_mm, dy_mm } => PosMode::Relative { dx_mm, dy_mm },
            PagePosMode::Absolute { x_mm, y_mm } => PosMode::Absolute { x_mm, y_mm },
        }),
        scale,
    };
    match execute_pos(
        rctx.project_root,
        page as u32,
        SlotExpr::single(slot as u32),
        &cfg,
    ) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(crate::task::BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(out) => crate::background::send_command_done(out.changed_state, vec![page], rctx),
    }
}
