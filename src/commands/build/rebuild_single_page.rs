use crate::{
    dto_models::{PhotoFile, PhotoGroup},
    run_solver,
    solver::{Request, RequestType},
};
use anyhow::Result;
use std::collections::HashMap;

/// Rebuilds a single page using the SinglePage solver.
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

    // Build PhotoGroup from the page's photo IDs
    let files: Vec<PhotoFile> = page
        .photos
        .iter()
        .filter_map(|id| photo_index.get(id).map(|(file, _)| file.clone()))
        .collect();

    if files.is_empty() {
        anyhow::bail!("Page {} has no valid photos", page_idx);
    }

    let group = PhotoGroup {
        group: format!("page_{}", page_idx),
        sort_key: String::new(),
        files,
    };

    // Run SinglePage solver
    let request = Request {
        request_type: RequestType::SinglePage,
        groups: &[group],
        config: &state.config.book_layout_solver,
        ga_config: &state.config.page_layout_solver,
        book_config: &state.config.book,
    };

    let result = run_solver(&request)?;

    if result.is_empty() {
        anyhow::bail!("Solver returned no result for page {}", page_idx);
    }

    state.layout[page_idx].slots = result[0].slots.clone();
    state.layout[page_idx].photos = result[0].photos.clone();

    Ok(())
}
