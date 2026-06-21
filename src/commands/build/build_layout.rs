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
use crate::models::{
    BookConfig, BookLayoutSolverConfig, LayoutPage, PageLayoutSolverConfig, PageMode, PhotoGroup,
    build_photo_index,
};
use anyhow::Result;
use tracing::warn;

#[allow(clippy::ptr_arg)]
pub(super) fn build_full_book(
    layout: &mut Vec<LayoutPage>,
    book_config: &BookConfig,
    solver_config: &BookLayoutSolverConfig,
    page_layout_config: &PageLayoutSolverConfig,
    photos: &[PhotoGroup],
) -> Result<Vec<usize>> {
    let layout_len = layout.len();
    if layout_len > 0 && book_config.cover.active {
        let effective_start = skip_cover_if_needed(true, 0, layout_len - 1)?;
        let groups = collect_photos_as_groups(layout, photos, effective_start, layout_len);
        return solve_multipage(
            layout,
            book_config,
            solver_config,
            page_layout_config,
            photos,
            &groups,
            Some((effective_start, layout_len)),
            None,
        );
    }
    solve_multipage(
        layout,
        book_config,
        solver_config,
        page_layout_config,
        photos,
        photos,
        None,
        None,
    )
}

#[allow(clippy::ptr_arg)]
pub(super) fn build_outdated_pages(
    layout: &mut Vec<LayoutPage>,
    pages: &[usize],
    book_config: &BookConfig,
    page_layout_config: &PageLayoutSolverConfig,
    photos: &[PhotoGroup],
) -> Result<Vec<usize>> {
    let photo_index = build_photo_index(photos);
    for &idx in pages {
        if layout[idx].mode != PageMode::Manual {
            solve_single_page(layout, idx, book_config, page_layout_config, &photo_index)?;
        }
    }
    Ok(pages.to_vec())
}

#[allow(clippy::ptr_arg)]
pub(super) fn build_page(
    layout: &mut Vec<LayoutPage>,
    idx: usize,
    book_config: &BookConfig,
    page_layout_config: &PageLayoutSolverConfig,
    photos: &[PhotoGroup],
) -> Result<Vec<usize>> {
    if layout.is_empty() {
        anyhow::bail!("No layout exists. Run `fotobuch build` first.");
    }
    if idx >= layout.len() {
        anyhow::bail!(
            "Invalid page index {} (layout has {} pages, indices 0..{})",
            idx,
            layout.len(),
            layout.len().saturating_sub(1),
        );
    }
    if layout[idx].mode == PageMode::Manual {
        anyhow::bail!(
            "Cannot rebuild page {}: page is in manual mode. \
             Use `page mode {} a` to switch to auto mode first.",
            idx,
            idx
        );
    }
    let photo_index = build_photo_index(photos);
    solve_single_page(layout, idx, book_config, page_layout_config, &photo_index)?;
    Ok(vec![idx])
}

#[allow(clippy::ptr_arg, clippy::too_many_arguments)]
pub(super) fn build_page_range(
    layout: &mut Vec<LayoutPage>,
    start: usize,
    end: usize,
    flex: usize,
    book_config: &BookConfig,
    solver_config: &BookLayoutSolverConfig,
    page_layout_config: &PageLayoutSolverConfig,
    photos: &[PhotoGroup],
) -> Result<Vec<usize>> {
    if layout.is_empty() {
        anyhow::bail!("No layout exists. Run `fotobuch build` first.");
    }
    if start > end || end >= layout.len() {
        anyhow::bail!(
            "Invalid page range {}-{} (layout has {} pages, indices 0..{})",
            start,
            end,
            layout.len(),
            layout.len().saturating_sub(1),
        );
    }
    let effective_start = skip_cover_if_needed(book_config.cover.active, start, end)?;
    let groups = collect_photos_as_groups(layout, photos, effective_start, end + 1);
    let n = end - effective_start + 1;
    let custom_config = BookLayoutSolverConfig {
        page_min: n.saturating_sub(flex).max(1),
        page_max: n + flex,
        page_target: n,
        ..solver_config.clone()
    };
    solve_multipage(
        layout,
        book_config,
        solver_config,
        page_layout_config,
        photos,
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
