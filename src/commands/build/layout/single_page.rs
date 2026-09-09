use super::cover_page::update_cover_page;
use crate::models::{LayoutPage, PhotoFile, PhotoGroup};
use crate::run_solver;
use crate::solver::{Request, RequestType};
use crate::state_manager::{ReadOnlyState, WriteLayoutState};
use anyhow::Result;
use std::collections::HashMap;

/// Solves a single page using either the deterministic cover solver (page 0,
/// non-`Free` mode) or the GA solver (all other cases).
///
/// # Arguments
/// * `page_idx` - **0-based** index into `layout` (e.g., 0 = first page, 1 = second page).
pub(super) fn solve_single_page(
    wls: &mut WriteLayoutState<'_>,
    page_idx: usize,
    photo_index: &HashMap<String, (PhotoFile, String)>,
) -> Result<()> {
    if page_idx >= wls.layout().len() {
        anyhow::bail!(
            "Page {} does not exist (layout has {} pages)",
            page_idx,
            wls.layout().len()
        );
    }

    let files: Vec<PhotoFile> = wls.layout()[page_idx]
        .photos
        .iter()
        .filter_map(|id| photo_index.get(id).map(|(file, _)| file.clone()))
        .collect();

    if files.is_empty() {
        anyhow::bail!("Page {} has no valid photos", page_idx);
    }

    if page_idx == 0 && wls.config().book.cover.active {
        update_cover_page(wls, photo_index)
    } else {
        solve_inner_page(wls, page_idx, files)
    }
}

// ── inner pages ───────────────────────────────────────────────────────────────

fn solve_inner_page(
    wls: &mut WriteLayoutState<'_>,
    page_idx: usize,
    files: Vec<PhotoFile>,
) -> Result<()> {
    let group = make_photo_group_for_page(page_idx, files);
    let book_config = wls.config().book.clone();
    let page_layout_config = wls.config().page_layout_solver.clone();
    let request = Request {
        request_type: RequestType::SinglePage,
        groups: &[group],
        page_layout_config: &page_layout_config,
        canvas_config: &book_config,
    };
    let result = run_solver(&request)?;
    apply_result(wls.layout_mut(), page_idx, result)
}

// ── shared helpers ────────────────────────────────────────────────────────────

fn make_photo_group_for_page(page_idx: usize, files: Vec<PhotoFile>) -> PhotoGroup {
    PhotoGroup {
        group: format!("page_{page_idx}"),
        sort_key: String::new(),
        files,
    }
}

fn apply_result(layout: &mut [LayoutPage], page_idx: usize, result: Vec<LayoutPage>) -> Result<()> {
    if result.is_empty() {
        anyhow::bail!("Solver returned no result for page {}", page_idx);
    }
    layout[page_idx].slots = result[0].slots.clone();
    layout[page_idx].photos = result[0].photos.clone();
    Ok(())
}
