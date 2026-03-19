//! Internal helpers shared by all page-command executors.

use crate::dto_models::LayoutPage;

use super::types::{DstSwap, SlotExpr, Src, ValidationError};

// ── Index resolution ──────────────────────────────────────────────────────────

/// Resolve a 1-based page number to a 0-based index, or return ValidationError.
pub(crate) fn page_idx(page: u32, layout: &[LayoutPage]) -> Result<usize, ValidationError> {
    if page == 0 || page as usize > layout.len() {
        return Err(ValidationError::PageNotFound(page));
    }
    Ok(page as usize - 1)
}

/// Resolve slot numbers on a page to 0-based indices and validate they exist.
/// `slots` are 1-based slot numbers.
pub(crate) fn resolve_slots(
    page: u32,
    slot_expr: &SlotExpr,
    layout: &[LayoutPage],
) -> Result<Vec<usize>, ValidationError> {
    let idx = page_idx(page, layout)?;
    let n_slots = layout[idx].photos.len();
    let mut result = Vec::with_capacity(slot_expr.slots.len());
    for &s in &slot_expr.slots {
        if s == 0 || s as usize > n_slots {
            return Err(ValidationError::SlotNotFound { page, slot: s });
        }
        result.push(s as usize - 1);
    }
    Ok(result)
}

/// Collect photo IDs at the given 0-based slot indices on a page.
pub(super) fn photos_at_slots(
    layout: &[LayoutPage],
    page_idx: usize,
    slot_indices: &[usize],
) -> Vec<String> {
    slot_indices
        .iter()
        .map(|&i| layout[page_idx].photos[i].clone())
        .collect()
}

/// Remove all pages with no photos from the layout.
/// Returns the 1-based page numbers that were deleted.
pub(crate) fn delete_empty_pages(layout: &mut Vec<LayoutPage>) -> Vec<u32> {
    let mut deleted = Vec::new();
    let mut i = 0;
    while i < layout.len() {
        if layout[i].photos.is_empty() {
            deleted.push(layout[i].page as u32);
            layout.remove(i);
        } else {
            i += 1;
        }
    }
    deleted
}

/// Remove photos at given 0-based slot indices from a page (descending order to keep indices stable).
pub(crate) fn remove_slots(
    layout: &mut [LayoutPage],
    page_idx: usize,
    mut slot_indices: Vec<usize>,
) {
    slot_indices.sort_unstable_by(|a, b| b.cmp(a));
    for i in slot_indices {
        layout[page_idx].photos.remove(i);
        if i < layout[page_idx].slots.len() {
            layout[page_idx].slots.remove(i);
        }
    }
}

// ── Photo collection ──────────────────────────────────────────────────────────

pub(super) fn collect_src_photos(
    src: &Src,
    layout: &[LayoutPage],
) -> Result<(Vec<String>, Vec<usize>), ValidationError> {
    match src {
        Src::Pages(pe) => {
            let mut photos = Vec::new();
            let mut indices = Vec::new();
            for &p in &pe.pages {
                let idx = page_idx(p, layout)?;
                photos.extend(layout[idx].photos.clone());
                indices.push(idx);
            }
            Ok((photos, indices))
        }
        Src::Slots { page, slots } => {
            let idx = page_idx(*page, layout)?;
            let slot_indices = resolve_slots(*page, slots, layout)?;
            let photos = photos_at_slots(layout, idx, &slot_indices);
            Ok((photos, vec![idx]))
        }
    }
}

/// Returns `(photos, page_idx, slot_indices_within_page)`.
/// For the `Pages` variant, `slot_indices` contains all indices `[0..n_photos)`.
pub(super) fn collect_src_photos_with_indices(
    src: &Src,
    layout: &[LayoutPage],
) -> Result<(Vec<String>, usize, Vec<usize>), ValidationError> {
    match src {
        Src::Pages(pe) => {
            let p = pe.pages[0];
            let idx = page_idx(p, layout)?;
            let all_slots: Vec<usize> = (0..layout[idx].photos.len()).collect();
            let photos = layout[idx].photos.clone();
            Ok((photos, idx, all_slots))
        }
        Src::Slots { page, slots } => {
            let idx = page_idx(*page, layout)?;
            let slot_indices = resolve_slots(*page, slots, layout)?;
            let photos = photos_at_slots(layout, idx, &slot_indices);
            Ok((photos, idx, slot_indices))
        }
    }
}

pub(super) fn collect_dst_swap_photos_with_indices(
    dst: &DstSwap,
    layout: &[LayoutPage],
) -> Result<(Vec<String>, usize, Vec<usize>), ValidationError> {
    match dst {
        DstSwap::Pages(pe) => {
            let p = pe.pages[0];
            let idx = page_idx(p, layout)?;
            let all_slots: Vec<usize> = (0..layout[idx].photos.len()).collect();
            let photos = layout[idx].photos.clone();
            Ok((photos, idx, all_slots))
        }
        DstSwap::Slots { page, slots } => {
            let idx = page_idx(*page, layout)?;
            let slot_indices = resolve_slots(*page, slots, layout)?;
            let photos = photos_at_slots(layout, idx, &slot_indices);
            Ok((photos, idx, slot_indices))
        }
    }
}

// ── Formatting helpers ────────────────────────────────────────────────────────

pub(super) fn format_src_desc(src: &Src) -> String {
    match src {
        Src::Pages(pe) => format_pages_list(&pe.pages),
        Src::Slots { page, slots } => {
            let slot_list: Vec<String> = slots.slots.iter().map(|s| s.to_string()).collect();
            format!("page {}:{}", page, slot_list.join(","))
        }
    }
}

pub(super) fn format_pages_list(pages: &[u32]) -> String {
    let list: Vec<String> = pages.iter().map(|p| p.to_string()).collect();
    list.join(",")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{SlotExpr, ValidationError};
    use super::super::test_fixtures::make_state_with_layout;

    #[test]
    fn test_page_idx_valid() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"], vec!["p1.jpg"]]);
        assert_eq!(page_idx(1, &state.layout).unwrap(), 0);
        assert_eq!(page_idx(2, &state.layout).unwrap(), 1);
    }

    #[test]
    fn test_page_idx_out_of_range() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"]]);
        assert_eq!(
            page_idx(2, &state.layout),
            Err(ValidationError::PageNotFound(2))
        );
        assert_eq!(
            page_idx(0, &state.layout),
            Err(ValidationError::PageNotFound(0))
        );
    }

    #[test]
    fn test_resolve_slots_valid() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg", "p2.jpg"]]);
        let slots = SlotExpr::from_range(1, 3);
        let indices = resolve_slots(1, &slots, &state.layout).unwrap();
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_resolve_slots_out_of_range() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"]]);
        let slots = SlotExpr::single(3);
        assert_eq!(
            resolve_slots(1, &slots, &state.layout),
            Err(ValidationError::SlotNotFound { page: 1, slot: 3 })
        );
    }
}
