//! Slot-range swapping for `page swap` (same-page or cross-page slots).

use crate::commands::page::helpers::{
    collect_dst_swap_photos_with_indices, collect_src_photos_with_indices,
};
use crate::commands::page::types::{DstSwap, PageMoveError, PageMoveResult, Src, ValidationError};
use crate::models::{LayoutPage, PageMode, PhotoGroup};
use crate::state_manager::{ReadOnlyState, WriteLayoutState};

use super::{dst_swap_is_contiguous, src_is_contiguous};

/// Swap the photos at two contiguous slot ranges (possibly across pages).
pub(super) fn swap_slots(
    s: &mut WriteLayoutState,
    left: Src,
    right: DstSwap,
) -> Result<(String, PageMoveResult), PageMoveError> {
    if !src_is_contiguous(&left) || !dst_swap_is_contiguous(&right) {
        return Err(ValidationError::SwapNonContiguous.into());
    }

    let (left_photos, left_page_idx, left_slot_indices) =
        collect_src_photos_with_indices(&left, s.layout())?;
    let (right_photos, right_page_idx, right_slot_indices) =
        collect_dst_swap_photos_with_indices(&right, s.layout())?;

    // Same page: slot ranges must not overlap.
    if left_page_idx == right_page_idx {
        let left_set: std::collections::HashSet<usize> =
            left_slot_indices.iter().copied().collect();
        if right_slot_indices.iter().any(|i| left_set.contains(i)) {
            return Err(ValidationError::SwapRangesOverlap.into());
        }
    }

    swap_photos_in_layout(
        s.layout_mut(),
        SwapSide {
            page_idx: left_page_idx,
            slot_indices: &left_slot_indices,
            photos: &left_photos,
        },
        SwapSide {
            page_idx: right_page_idx,
            slot_indices: &right_slot_indices,
            photos: &right_photos,
        },
    );

    // On a Manual page the receiving slots keep their position and width, but their
    // height adapts to the incoming photo's aspect ratio (cross-page swaps only).
    if left_page_idx != right_page_idx {
        let left_recv = right_photos.len();
        let right_recv = left_photos.len();
        let left_start = left_slot_indices.iter().min().copied().unwrap_or(0);
        let right_start = right_slot_indices.iter().min().copied().unwrap_or(0);
        let photos = s.photos().to_vec();
        adapt_manual_slot_ratios(
            s.layout_mut(),
            &photos,
            left_page_idx,
            left_start,
            left_recv,
        );
        adapt_manual_slot_ratios(
            s.layout_mut(),
            &photos,
            right_page_idx,
            right_start,
            right_recv,
        );
    }

    let mut modified_pages = vec![left_page_idx as u32, right_page_idx as u32];
    modified_pages.sort();
    modified_pages.dedup();

    Ok((
        "page swap".to_string(),
        PageMoveResult {
            pages_modified: modified_pages,
            pages_inserted: vec![],
            pages_deleted: vec![],
        },
    ))
}

struct SwapSide<'a> {
    page_idx: usize,
    slot_indices: &'a [usize],
    photos: &'a [String],
}

fn swap_photos_in_layout(layout: &mut [LayoutPage], left: SwapSide, right: SwapSide) {
    let swap_slots = left.photos.len() != right.photos.len();

    let mut left_desc: Vec<usize> = left.slot_indices.to_vec();
    left_desc.sort_unstable_by(|a, b| b.cmp(a));
    for &i in &left_desc {
        layout[left.page_idx].photos.remove(i);
        if swap_slots && i < layout[left.page_idx].slots.len() {
            layout[left.page_idx].slots.remove(i);
        }
    }

    let insert_at = left.slot_indices.iter().min().copied().unwrap_or(0);
    for (j, photo) in right.photos.iter().enumerate() {
        let pos = (insert_at + j).min(layout[left.page_idx].photos.len());
        layout[left.page_idx].photos.insert(pos, photo.clone());
    }

    let mut right_desc: Vec<usize> = right.slot_indices.to_vec();
    right_desc.sort_unstable_by(|a, b| b.cmp(a));
    for &i in &right_desc {
        layout[right.page_idx].photos.remove(i);
        if swap_slots && i < layout[right.page_idx].slots.len() {
            layout[right.page_idx].slots.remove(i);
        }
    }

    let insert_at_r = right.slot_indices.iter().min().copied().unwrap_or(0);
    for (j, photo) in left.photos.iter().enumerate() {
        let pos = (insert_at_r + j).min(layout[right.page_idx].photos.len());
        layout[right.page_idx].photos.insert(pos, photo.clone());
    }
}

fn photo_pixel_size(photos: &[PhotoGroup], id: &str) -> Option<(u32, u32)> {
    photos
        .iter()
        .flat_map(|g| g.files.iter())
        .find(|f| f.id == id)
        .map(|f| (f.width_px, f.height_px))
}

/// On a Manual page, adapt each receiving slot's height to the incoming photo's ratio.
fn adapt_manual_slot_ratios(
    layout: &mut [LayoutPage],
    photos: &[PhotoGroup],
    page_idx: usize,
    start: usize,
    count: usize,
) {
    if layout[page_idx].mode != PageMode::Manual {
        return;
    }
    for i in start..start + count {
        let Some(photo_id) = layout[page_idx].photos.get(i).cloned() else {
            continue;
        };
        if layout[page_idx].slots.get(i).is_none() {
            continue;
        }
        if let Some((w_px, h_px)) = photo_pixel_size(photos, &photo_id)
            && w_px > 0
        {
            let slot = &mut layout[page_idx].slots[i];
            slot.height_mm = slot.width_mm * (h_px as f64 / w_px as f64);
        }
    }
}
