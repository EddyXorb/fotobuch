use super::cover_page::update_cover_page;
use crate::models::{BookConfig, LayoutPage, PageLayoutSolverConfig, PhotoFile, PhotoGroup};
use crate::run_solver;
use crate::solver::{Request, RequestType};
use anyhow::Result;
use std::collections::HashMap;

/// Solves a single page using either the deterministic cover solver (page 0,
/// non-`Free` mode) or the GA solver (all other cases).
///
/// # Arguments
/// * `page_idx` - **0-based** index into `layout` (e.g., 0 = first page, 1 = second page).
pub(super) fn solve_single_page(
    layout: &mut [LayoutPage],
    page_idx: usize,
    book_config: &BookConfig,
    page_layout_config: &PageLayoutSolverConfig,
    photo_index: &HashMap<String, (PhotoFile, String)>,
) -> Result<()> {
    if page_idx >= layout.len() {
        anyhow::bail!(
            "Page {} does not exist (layout has {} pages)",
            page_idx,
            layout.len()
        );
    }

    let page = &layout[page_idx];

    let files: Vec<PhotoFile> = page
        .photos
        .iter()
        .filter_map(|id| photo_index.get(id).map(|(file, _)| file.clone()))
        .collect();

    if files.is_empty() {
        anyhow::bail!("Page {} has no valid photos", page_idx);
    }

    if page_idx == 0 && book_config.cover.active {
        update_cover_page(layout, book_config, page_layout_config, photo_index)
    } else {
        solve_inner_page(layout, page_idx, book_config, page_layout_config, files)
    }
}

// ── inner pages ───────────────────────────────────────────────────────────────

fn solve_inner_page(
    layout: &mut [LayoutPage],
    page_idx: usize,
    book_config: &BookConfig,
    page_layout_config: &PageLayoutSolverConfig,
    files: Vec<PhotoFile>,
) -> Result<()> {
    let group = photo_group_for_page(page_idx, files);
    let request = Request {
        request_type: RequestType::SinglePage,
        groups: &[group],
        page_layout_config,
        canvas_config: book_config,
    };
    let result = run_solver(&request)?;
    apply_result(layout, page_idx, result)
}

// ── shared helpers ────────────────────────────────────────────────────────────

fn photo_group_for_page(page_idx: usize, files: Vec<PhotoFile>) -> PhotoGroup {
    PhotoGroup {
        group: format!("page_{page_idx}"),
        sort_key: String::new(),
        files,
    }
}

fn apply_result(
    layout: &mut [LayoutPage],
    page_idx: usize,
    result: Vec<LayoutPage>,
) -> Result<()> {
    if result.is_empty() {
        anyhow::bail!("Solver returned no result for page {}", page_idx);
    }
    layout[page_idx].slots = result[0].slots.clone();
    layout[page_idx].photos = result[0].photos.clone();
    Ok(())
}
