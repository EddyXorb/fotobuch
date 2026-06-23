//! Manual-page move: drop slots onto a positioned Manual page.

use std::path::Path;

use crate::commands::page::helpers::{
    delete_empty_pages, page_idx, photos_at_slots, resolve_slots,
};
use crate::commands::page::types::{PageMoveError, PageMoveResult, SlotExpr, Src, ValidationError};
use crate::commands::{CommandOutput, run_write_command};
use crate::models::{PageMode, Slot};
use crate::state_manager::{ReadOnlyState, WriteLayoutState};

/// Cascade offset between consecutive slots dropped onto a Manual page in one gesture.
const MANUAL_DROP_CASCADE_MM: f64 = 10.0;

pub(super) fn execute_move_to_manual(
    project_root: &Path,
    src: Src,
    dst_page: u32,
    x_mm: f64,
    y_mm: f64,
) -> Result<CommandOutput<PageMoveResult>, PageMoveError> {
    let Src::Slots {
        page: src_page,
        slots,
    } = src
    else {
        return Err(ValidationError::PageNotManual(dst_page).into());
    };

    run_write_command(project_root, |mgr| {
        let mut view = mgr.get_write_layout_state();
        apply_move_to_manual(&mut view, src_page, &slots, dst_page, x_mm, y_mm)
    })
}

/// Mutating core of `execute_move_to_manual`, working on the layout view.
fn apply_move_to_manual(
    s: &mut WriteLayoutState,
    src_page: u32,
    slots: &SlotExpr,
    dst_page: u32,
    x_mm: f64,
    y_mm: f64,
) -> Result<(String, PageMoveResult), PageMoveError> {
    let dst_idx = page_idx(dst_page, s.layout())?;
    if s.layout()[dst_idx].mode != PageMode::Manual {
        return Err(ValidationError::PageNotManual(dst_page).into());
    }
    if src_page == dst_page {
        return Ok((String::new(), super::empty_result()));
    }

    let src_idx = page_idx(src_page, s.layout())?;
    let mut slot_indices = resolve_slots(src_page, slots, s.layout())?;
    slot_indices.sort_unstable();

    // Snapshot each moved photo together with the size of its source slot.
    let mut moved: Vec<(String, f64, f64)> = Vec::with_capacity(slot_indices.len());
    for &i in &slot_indices {
        let photo = photos_at_slots(s.layout(), src_idx, &[i])?.remove(0);
        let slot_dims = s.layout()[src_idx]
            .slots
            .get(i)
            .map(|sl| (sl.width_mm, sl.height_mm));
        let (w, h) = slot_dims.unwrap_or_else(|| default_manual_slot_size(s.state(), dst_idx));
        moved.push((photo, w, h));
    }

    // Remove from src descending so indices stay valid.
    let src_is_manual = s.layout()[src_idx].mode == PageMode::Manual;
    let mut desc = slot_indices.clone();
    desc.sort_unstable_by(|a, b| b.cmp(a));
    for &i in &desc {
        s.layout_mut()[src_idx].photos.remove(i);
        if src_is_manual && i < s.layout()[src_idx].slots.len() {
            s.layout_mut()[src_idx].slots.remove(i);
        }
    }

    // Append photos and matching positioned slots to the manual destination.
    for (n, (photo, w, h)) in moved.into_iter().enumerate() {
        let offset = MANUAL_DROP_CASCADE_MM * n as f64;
        s.layout_mut()[dst_idx].photos.push(photo);
        s.layout_mut()[dst_idx].slots.push(Slot {
            x_mm: x_mm + offset,
            y_mm: y_mm + offset,
            width_mm: w,
            height_mm: h,
        });
    }

    let dst_page_num = dst_idx as u32;
    let deleted = delete_empty_pages(s.layout_mut());
    let mut modified = vec![src_page, dst_page_num];
    modified.retain(|p| !deleted.contains(p));
    modified.sort_unstable();
    modified.dedup();

    Ok((
        format!("page move: slots from page {src_page} -> manual page {dst_page_num}"),
        PageMoveResult {
            pages_modified: modified,
            pages_inserted: vec![],
            pages_deleted: deleted,
        },
    ))
}

/// Fallback slot size when a source slot has no computed geometry yet: 30 % of destination page.
fn default_manual_slot_size(state: &crate::models::ProjectState, dst_idx: usize) -> (f64, f64) {
    let (pw, ph) = state.page_dimensions_mm(dst_idx);
    (pw * 0.3, ph * 0.3)
}
