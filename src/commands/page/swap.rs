//! `fotobuch page swap` command.
//!
//! Dispatches to one of two strategies:
//! - [`page_blocks`] — transpose whole contiguous page ranges (Pages × Pages).
//! - [`slots`] — swap the photos at two contiguous slot ranges.

mod page_blocks;
mod slots;

use std::path::Path;

use crate::commands::{CommandOutput, run_write_command};
use crate::state_manager::WriteLayoutState;

use super::types::{DstSwap, PageMoveError, PageMoveResult, Src};

use page_blocks::swap_page_blocks;
use slots::swap_slots;

pub fn execute_swap(
    project_root: &Path,
    left: Src,
    right: DstSwap,
) -> Result<CommandOutput<PageMoveResult>, PageMoveError> {
    run_write_command(project_root, |mgr| {
        let mut view = mgr.get_write_layout_state();
        apply_swap(&mut view, left, right)
    })
}

/// Dispatch a swap to the page-block or slot variant on the layout view.
fn apply_swap(
    s: &mut WriteLayoutState,
    left: Src,
    right: DstSwap,
) -> Result<(String, PageMoveResult), PageMoveError> {
    // Pages × Pages — block transposition, contiguous ranges only.
    if let (Src::Pages(lpe), DstSwap::Pages(rpe)) = (&left, &right) {
        return swap_page_blocks(s, &lpe.pages, &rpe.pages);
    }

    // Otherwise: blockwise slot replacement, contiguous ranges only.
    swap_slots(s, left, right)
}

// ── Contiguity helpers shared by both strategies ───────────────────────────────

fn is_contiguous(items: &[u32]) -> bool {
    items.len() <= 1 || items.windows(2).all(|w| w[1] == w[0] + 1)
}

fn src_is_contiguous(src: &Src) -> bool {
    match src {
        Src::Pages(pe) => is_contiguous(&pe.pages),
        Src::Slots { slots, .. } => slots.items.len() <= 1,
    }
}

fn dst_swap_is_contiguous(dst: &DstSwap) -> bool {
    match dst {
        DstSwap::Pages(pe) => is_contiguous(&pe.pages),
        DstSwap::Slots { slots, .. } => slots.items.len() <= 1,
    }
}
