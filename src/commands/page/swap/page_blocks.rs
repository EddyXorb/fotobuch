//! Page-block transposition for `page swap` (Pages × Pages).

use crate::commands::page::helpers::page_idx;
use crate::commands::page::types::{PageMoveError, PageMoveResult, ValidationError};
use crate::models::LayoutPage;
use crate::state_manager::{ReadOnlyState, WriteLayoutState};

use super::is_contiguous;

/// Transpose two contiguous, non-overlapping page ranges.
pub(super) fn swap_page_blocks(
    s: &mut WriteLayoutState,
    left_pages: &[u32],
    right_pages: &[u32],
) -> Result<(String, PageMoveResult), PageMoveError> {
    if !is_contiguous(left_pages) || !is_contiguous(right_pages) {
        return Err(ValidationError::SwapNonContiguous.into());
    }

    let left_set: std::collections::HashSet<u32> = left_pages.iter().copied().collect();
    if right_pages.iter().any(|p| left_set.contains(p)) {
        return Err(ValidationError::SwapRangesOverlap.into());
    }

    for &p in left_pages.iter().chain(right_pages.iter()) {
        page_idx(p, s.layout())?;
    }

    let mut modified_pages: Vec<u32> = left_pages
        .iter()
        .chain(right_pages.iter())
        .copied()
        .collect();
    modified_pages.sort();
    modified_pages.dedup();

    block_transpose_pages(s.layout_mut(), left_pages, right_pages);

    Ok((
        "page swap".to_string(),
        PageMoveResult {
            pages_modified: modified_pages,
            pages_inserted: vec![],
            pages_deleted: vec![],
        },
    ))
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
