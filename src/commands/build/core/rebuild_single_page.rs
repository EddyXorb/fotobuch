use super::super::cover::update_cover_page;
use crate::dto_models::{PhotoFile, PhotoGroup};
use crate::run_solver;
use crate::solver::{Request, RequestType};
use anyhow::Result;
use std::collections::HashMap;

/// Rebuilds a single page using either the deterministic cover solver (page 0,
/// non-`Free` mode) or the GA solver (all other cases).
///
/// # Arguments
/// * `page_idx` - **0-based** index into `state.layout` (e.g., 0 = first page, 1 = second page).
///   This does NOT consider the `page_nr` field in the layout.
pub fn rebuild_single_page(
    state: &mut crate::dto_models::ProjectState,
    page_idx: usize,
    photo_index: &HashMap<String, (PhotoFile, String)>,
) -> Result<()> {
    if page_idx >= state.layout.len() {
        anyhow::bail!(
            "Page {} does not exist (layout has {} pages)",
            page_idx,
            state.layout.len()
        );
    }

    let page = &state.layout[page_idx];

    let files: Vec<PhotoFile> = page
        .photos
        .iter()
        .filter_map(|id| photo_index.get(id).map(|(file, _)| file.clone()))
        .collect();

    if files.is_empty() {
        anyhow::bail!("Page {} has no valid photos", page_idx);
    }

    if page_idx == 0 && state.has_cover() {
        update_cover_page(state, photo_index)
    } else {
        rebuild_inner_page(state, page_idx, files)
    }
}

// ── inner pages ───────────────────────────────────────────────────────────────

fn rebuild_inner_page(
    state: &mut crate::dto_models::ProjectState,
    page_idx: usize,
    files: Vec<PhotoFile>,
) -> Result<()> {
    let group = photo_group_for_page(page_idx, files);
    let request = Request {
        request_type: RequestType::SinglePage,
        groups: &[group],
        config: &state.config.book_layout_solver,
        ga_config: &state.config.page_layout_solver,
        canvas_config: &state.config.book,
    };
    let result = run_solver(&request)?;
    apply_result(state, page_idx, result)
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
    state: &mut crate::dto_models::ProjectState,
    page_idx: usize,
    result: Vec<crate::dto_models::LayoutPage>,
) -> Result<()> {
    if result.is_empty() {
        anyhow::bail!("Solver returned no result for page {}", page_idx);
    }
    state.layout[page_idx].slots = result[0].slots.clone();
    state.layout[page_idx].photos = result[0].photos.clone();
    Ok(())
}
