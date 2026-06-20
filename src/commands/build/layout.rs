use super::helpers::collect_photos_as_groups;
use super::solve::multipage::solve_multipage;
use super::solve::single_page::solve_single_page;
use crate::dto_models::{BookLayoutSolverConfig, PageMode, PhotoGroup, build_photo_index};
use crate::state_manager::StateManager;
use anyhow::Result;
use tracing::warn;

pub(super) fn build_full_book(mgr: &mut StateManager) -> Result<Vec<usize>> {
    let layout_len = mgr.state.layout.len();
    // For an existing layout with an active cover, skip page 0 so the cover is not
    // redistributed (use rebuild --page 0 to rebuild it explicitly).
    if layout_len > 0 && mgr.state.has_cover() {
        let effective_start = skip_cover_if_needed(true, 0, layout_len - 1)?;
        let groups = collect_photos_as_groups(&mgr.state, effective_start, layout_len);
        return solve_multipage(
            &mut mgr.state,
            &groups,
            Some((effective_start, layout_len)),
            None,
        );
    }
    let groups: Vec<PhotoGroup> = mgr.state.photos.clone();
    solve_multipage(&mut mgr.state, &groups, None, None)
}

pub(super) fn build_outdated_pages(
    mgr: &mut StateManager,
    page_filter: Option<&[usize]>,
) -> Result<Vec<usize>> {
    let mut pages = mgr.outdated_pages_indices();
    if let Some(filter) = page_filter {
        pages.retain(|p| filter.contains(p));
    }
    let photo_index = build_photo_index(&mgr.state.photos);
    for &idx in &pages {
        if mgr.state.layout[idx].mode != PageMode::Manual {
            solve_single_page(&mut mgr.state, idx, &photo_index)?;
        }
    }
    Ok(pages)
}

pub(super) fn build_page(mgr: &mut StateManager, idx: usize) -> Result<Vec<usize>> {
    if mgr.state.layout.is_empty() {
        anyhow::bail!("No layout exists. Run `fotobuch build` first.");
    }
    if idx >= mgr.state.layout.len() {
        anyhow::bail!(
            "Invalid page index {} (layout has {} pages, indices 0..{})",
            idx,
            mgr.state.layout.len(),
            mgr.state.layout.len().saturating_sub(1),
        );
    }
    if mgr.state.layout[idx].mode == PageMode::Manual {
        anyhow::bail!(
            "Cannot rebuild page {}: page is in manual mode. \
             Use `page mode {} a` to switch to auto mode first.",
            idx,
            idx
        );
    }
    let photo_index = build_photo_index(&mgr.state.photos);
    solve_single_page(&mut mgr.state, idx, &photo_index)?;
    Ok(vec![idx])
}

pub(super) fn build_page_range(
    mgr: &mut StateManager,
    start: usize,
    end: usize,
    flex: usize,
) -> Result<Vec<usize>> {
    if mgr.state.layout.is_empty() {
        anyhow::bail!("No layout exists. Run `fotobuch build` first.");
    }
    if start > end || end >= mgr.state.layout.len() {
        anyhow::bail!(
            "Invalid page range {}-{} (layout has {} pages, indices 0..{})",
            start,
            end,
            mgr.state.layout.len(),
            mgr.state.layout.len().saturating_sub(1),
        );
    }
    let effective_start = skip_cover_if_needed(mgr.state.has_cover(), start, end)?;
    let groups = collect_photos_as_groups(&mgr.state, effective_start, end + 1);
    let n = end - effective_start + 1;
    let custom_config = BookLayoutSolverConfig {
        page_min: n.saturating_sub(flex).max(1),
        page_max: n + flex,
        page_target: n,
        ..mgr.state.config.book_layout_solver.clone()
    };
    solve_multipage(
        &mut mgr.state,
        &groups,
        Some((effective_start, end + 1)),
        Some(custom_config),
    )
}

/// If cover is active and `start` is 0, skips the cover and returns effective start = 1.
/// Emits a warning. Returns `Err` if the resulting range would be empty.
pub(super) fn skip_cover_if_needed(has_cover: bool, start: usize, end: usize) -> Result<usize> {
    if !has_cover || start != 0 {
        return Ok(start);
    }
    warn!(
        "Cover page (index 0) is excluded from this rebuild. \
         Use `rebuild --page 0` to rebuild it explicitly."
    );
    if end == 0 {
        anyhow::bail!(
            "Range 0-0 contains only the cover page. \
             Use `rebuild --page 0` to rebuild it explicitly."
        );
    }
    Ok(1)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_cover_no_cover_returns_start_unchanged() {
        assert_eq!(skip_cover_if_needed(false, 0, 3).unwrap(), 0);
        assert_eq!(skip_cover_if_needed(false, 2, 5).unwrap(), 2);
    }

    #[test]
    fn skip_cover_has_cover_nonzero_start_unchanged() {
        assert_eq!(skip_cover_if_needed(true, 2, 5).unwrap(), 2);
    }

    #[test]
    fn skip_cover_has_cover_start_zero_returns_one() {
        assert_eq!(skip_cover_if_needed(true, 0, 3).unwrap(), 1);
    }

    #[test]
    fn skip_cover_range_zero_zero_errors() {
        assert!(skip_cover_if_needed(true, 0, 0).is_err());
    }
}
