//! `fotobuch page swap` command.

use std::path::Path;

use crate::commands::CommandOutput;
use crate::models::{LayoutPage, PageMode};
use crate::state_manager::StateManager;

use super::helpers::{
    collect_dst_swap_photos_with_indices, collect_src_photos_with_indices, page_idx,
};
use super::types::{DstSwap, PageMoveError, PageMoveResult, Src, ValidationError};

pub fn execute_swap(
    project_root: &Path,
    left: Src,
    right: DstSwap,
) -> Result<CommandOutput<PageMoveResult>, PageMoveError> {
    let mut mgr = StateManager::open(project_root)?;

    // Pages × Pages — block transposition, contiguous ranges only.
    if let (Src::Pages(lpe), DstSwap::Pages(rpe)) = (&left, &right) {
        if !is_contiguous(&lpe.pages) || !is_contiguous(&rpe.pages) {
            return Err(ValidationError::SwapNonContiguous.into());
        }

        let left_set: std::collections::HashSet<u32> = lpe.pages.iter().copied().collect();
        if rpe.pages.iter().any(|p| left_set.contains(p)) {
            return Err(ValidationError::SwapRangesOverlap.into());
        }

        for &p in lpe.pages.iter().chain(rpe.pages.iter()) {
            page_idx(p, &mgr.state.layout)?;
        }

        let mut modified_pages: Vec<u32> =
            lpe.pages.iter().chain(rpe.pages.iter()).copied().collect();
        modified_pages.sort();
        modified_pages.dedup();

        block_transpose_pages(&mut mgr.state.layout, &lpe.pages, &rpe.pages);
        let changed_state = mgr.finish("page swap")?;
        return Ok(CommandOutput {
            result: PageMoveResult {
                pages_modified: modified_pages,
                pages_inserted: vec![],
                pages_deleted: vec![],
            },
            changed_state,
        });
    }

    // Slot swap — blockwise replacement, contiguous ranges only.
    if !src_is_contiguous(&left) || !dst_swap_is_contiguous(&right) {
        return Err(ValidationError::SwapNonContiguous.into());
    }

    let (left_photos, left_page_idx, left_slot_indices) =
        collect_src_photos_with_indices(&left, &mgr.state.layout)?;
    let (right_photos, right_page_idx, right_slot_indices) =
        collect_dst_swap_photos_with_indices(&right, &mgr.state.layout)?;

    // Same page: slot ranges must not overlap.
    if left_page_idx == right_page_idx {
        let left_set: std::collections::HashSet<usize> =
            left_slot_indices.iter().copied().collect();
        if right_slot_indices.iter().any(|i| left_set.contains(i)) {
            return Err(ValidationError::SwapRangesOverlap.into());
        }
    }

    swap_photos_in_layout(
        &mut mgr.state.layout,
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
        let photos = mgr.state.photos.to_vec();
        adapt_manual_slot_ratios(
            &mut mgr.state.layout,
            &photos,
            left_page_idx,
            left_start,
            left_recv,
        );
        adapt_manual_slot_ratios(
            &mut mgr.state.layout,
            &photos,
            right_page_idx,
            right_start,
            right_recv,
        );
    }

    let mut modified_pages = vec![left_page_idx as u32, right_page_idx as u32];
    modified_pages.sort();
    modified_pages.dedup();

    let changed_state = mgr.finish("page swap")?;

    Ok(CommandOutput {
        result: PageMoveResult {
            pages_modified: modified_pages,
            pages_inserted: vec![],
            pages_deleted: vec![],
        },
        changed_state,
    })
}

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

/// Block-transpose two contiguous page ranges within the layout.
fn block_transpose_pages(layout: &mut Vec<LayoutPage>, left_pages: &[u32], right_pages: &[u32]) {
    let l0 = page_idx(left_pages[0], layout).unwrap();
    let l1 = page_idx(*left_pages.last().unwrap(), layout).unwrap();
    let r0 = page_idx(right_pages[0], layout).unwrap();
    let r1 = page_idx(*right_pages.last().unwrap(), layout).unwrap();

    // Normalize so that (l0..=l1) comes before (r0..=r1).
    let (l0, l1, r0, r1) = if l0 <= r0 {
        (l0, l1, r0, r1)
    } else {
        (r0, r1, l0, l1)
    };

    let segment: Vec<LayoutPage> = layout.drain(l0..=r1).collect();
    let left_len = l1 - l0 + 1;
    let right_start = r0 - l0;
    let right_len = r1 - r0 + 1;

    let mut new_segment = Vec::with_capacity(segment.len());
    new_segment.extend_from_slice(&segment[right_start..right_start + right_len]);
    new_segment.extend_from_slice(&segment[left_len..right_start]);
    new_segment.extend_from_slice(&segment[..left_len]);

    for (i, page) in new_segment.into_iter().enumerate() {
        layout.insert(l0 + i, page);
    }
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

fn photo_pixel_size(photos: &[crate::models::PhotoGroup], id: &str) -> Option<(u32, u32)> {
    photos
        .iter()
        .flat_map(|g| g.files.iter())
        .find(|f| f.id == id)
        .map(|f| (f.width_px, f.height_px))
}

/// On a Manual page, adapt each receiving slot's height to the incoming photo's ratio.
fn adapt_manual_slot_ratios(
    layout: &mut [crate::models::LayoutPage],
    photos: &[crate::models::PhotoGroup],
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
