//! Internal helpers shared by all page-command executors.

use crate::dto_models::LayoutPage;

use super::types::{DstSwap, SlotExpr, SlotItem, Src, ValidationError};

// ── Index resolution ──────────────────────────────────────────────────────────

/// Resolve a page number to a 0-based layout index, or return ValidationError.
/// Page 0 is valid when a cover page exists as the first layout entry.
pub(crate) fn page_idx(page: u32, layout: &[LayoutPage]) -> Result<usize, ValidationError> {
    layout
        .iter()
        .position(|p| p.page == page as usize)
        .ok_or(ValidationError::PageNotFound(page))
}

/// Resolve slot numbers on a page to 0-based indices and validate they exist.
/// Slot numbers are 1-based; open-ended ranges are resolved against the actual page size.
pub(crate) fn resolve_slots(
    page: u32,
    slot_expr: &SlotExpr,
    layout: &[LayoutPage],
) -> Result<Vec<usize>, ValidationError> {
    let idx = page_idx(page, layout)?;
    let n_slots = layout[idx].slots.len();
    let mut result = Vec::new();
    for item in &slot_expr.items {
        match item {
            SlotItem::Single(s) => {
                if *s == 0 || *s as usize > n_slots {
                    return Err(ValidationError::SlotNotFound { page, slot: *s });
                }
                result.push(*s as usize - 1);
            }
            SlotItem::Range { from, to } => {
                let start = from.unwrap_or(1);
                let end = to.unwrap_or(n_slots as u32);
                if start == 0 {
                    return Err(ValidationError::SlotNotFound { page, slot: 0 });
                }
                if end as usize > n_slots {
                    return Err(ValidationError::SlotNotFound { page, slot: end });
                }
                for s in start..=end {
                    result.push(s as usize - 1);
                }
            }
        }
    }
    Ok(result)
}

/// Collect photo IDs at the given 0-based slot indices on a page.
/// Returns error if any slot has no photo (index out of bounds in photos array).
pub(super) fn photos_at_slots(
    layout: &[LayoutPage],
    page_idx: usize,
    slot_indices: &[usize],
) -> Result<Vec<String>, ValidationError> {
    let mut photos = Vec::new();
    for &i in slot_indices {
        if i >= layout[page_idx].photos.len() {
            return Err(ValidationError::SlotEmpty {
                page: layout[page_idx].page as u32,
                slot: i as u32 + 1,
            });
        }
        photos.push(layout[page_idx].photos[i].clone());
    }
    Ok(photos)
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
            let photos = photos_at_slots(layout, idx, &slot_indices)?;
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
            let photos = photos_at_slots(layout, idx, &slot_indices)?;
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
            let photos = photos_at_slots(layout, idx, &slot_indices)?;
            Ok((photos, idx, slot_indices))
        }
    }
}

// ── Formatting helpers ────────────────────────────────────────────────────────

pub(super) fn format_src_desc(src: &Src) -> String {
    match src {
        Src::Pages(pe) => format_pages_list(&pe.pages),
        Src::Slots { page, slots } => {
            format!("page {}:{}", page, format_slot_expr(slots))
        }
    }
}

fn format_slot_expr(slots: &SlotExpr) -> String {
    let parts: Vec<String> = slots.items.iter().map(|item| match item {
        SlotItem::Single(n) => n.to_string(),
        SlotItem::Range { from, to } => match (from, to) {
            (Some(a), Some(b)) => format!("{a}..{b}"),
            (Some(a), None) => format!("{a}.."),
            (None, Some(b)) => format!("..{b}"),
            (None, None) => "..".to_string(),
        },
    }).collect();
    parts.join(",")
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
        // bounded range
        assert_eq!(resolve_slots(1, &SlotExpr::from_range(1, 3), &state.layout).unwrap(), vec![0, 1, 2]);
        // open end: 2.. → slots 2 and 3
        assert_eq!(resolve_slots(1, &SlotExpr::from_open_end(2), &state.layout).unwrap(), vec![1, 2]);
        // open start: ..2 → slots 1 and 2
        assert_eq!(resolve_slots(1, &SlotExpr::from_open_start(2), &state.layout).unwrap(), vec![0, 1]);
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
