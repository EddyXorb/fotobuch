//! Layout builders: translate a `BuildPlan` scope into solver-engine calls.
//!
//! This is the abstraction layer below `plan.rs`. The solver engines it drives
//! live in the child modules below and turn a set of photos into laid-out pages:
//! `single_page` and `multi_page` are the GA engines, `cover_page` the
//! deterministic cover engine. They are private to this layer.

mod cover_page;
mod multi_page;
mod single_page;

use self::multi_page::solve_multipage;
use self::single_page::solve_single_page;
use super::helpers::collect_photos_as_groups;
use crate::models::{PageMode, build_photo_index};
use crate::state_manager::{ReadOnlyState, WriteLayoutState};
use anyhow::Result;
use tracing::warn;

pub(super) fn build_full_book(wls: &mut WriteLayoutState<'_>) -> Result<Vec<usize>> {
    let layout_len = wls.layout().len();
    if layout_len > 0 && wls.config().book.cover.active {
        let effective_start = skip_cover_if_needed(true, 0, layout_len - 1)?;
        let groups = collect_photos_as_groups(wls.state(), effective_start, layout_len);
        return solve_multipage(wls, &groups, Some((effective_start, layout_len)), None);
    }
    let groups = wls.photos().to_vec();
    solve_multipage(wls, &groups, None, None)
}

pub(super) fn build_outdated_pages(
    wls: &mut WriteLayoutState<'_>,
    pages: &[usize],
) -> Result<Vec<usize>> {
    let photo_index = build_photo_index(wls.photos());
    for &idx in pages {
        if wls.layout()[idx].mode != PageMode::Manual {
            solve_single_page(wls, idx, &photo_index)?;
        }
    }
    Ok(pages.to_vec())
}

pub(super) fn build_page(wls: &mut WriteLayoutState<'_>, idx: usize) -> Result<Vec<usize>> {
    if wls.layout().is_empty() {
        anyhow::bail!("No layout exists. Run `fotobuch build` first.");
    }
    if idx >= wls.layout().len() {
        anyhow::bail!(
            "Invalid page index {} (layout has {} pages, indices 0..{})",
            idx,
            wls.layout().len(),
            wls.layout().len().saturating_sub(1),
        );
    }
    if wls.layout()[idx].mode == PageMode::Manual {
        anyhow::bail!(
            "Cannot rebuild page {}: page is in manual mode. \
             Use `page mode {} a` to switch to auto mode first.",
            idx,
            idx
        );
    }
    let photo_index = build_photo_index(wls.photos());
    solve_single_page(wls, idx, &photo_index)?;
    Ok(vec![idx])
}

pub(super) fn build_page_range(
    wls: &mut WriteLayoutState<'_>,
    start: usize,
    end: usize,
    flex: usize,
) -> Result<Vec<usize>> {
    if wls.layout().is_empty() {
        anyhow::bail!("No layout exists. Run `fotobuch build` first.");
    }
    if start > end || end >= wls.layout().len() {
        anyhow::bail!(
            "Invalid page range {}-{} (layout has {} pages, indices 0..{})",
            start,
            end,
            wls.layout().len(),
            wls.layout().len().saturating_sub(1),
        );
    }
    let effective_start = skip_cover_if_needed(wls.config().book.cover.active, start, end)?;
    let solver_config = wls.config().book_layout_solver.clone();
    let groups = collect_photos_as_groups(wls.state(), effective_start, end + 1);
    let n = end - effective_start + 1;
    let custom_config = crate::models::BookLayoutSolverConfig {
        page_min: n.saturating_sub(flex).max(1),
        page_max: n + flex,
        page_target: n,
        ..solver_config
    };
    solve_multipage(
        wls,
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
