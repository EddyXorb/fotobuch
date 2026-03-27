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

    let result = if page_idx == 0 && state.has_cover() {
        let cover = &state.config.book.cover;
        let inner_page_count = state.layout.len() - 1;
        let spread_config = CoverSpreadConfig {
            cover,
            inner_page_count,
        };
        let request = Request {
            request_type: RequestType::SinglePage,
            groups: &[group],
            config: &state.config.book_layout_solver,
            ga_config: &state.config.page_layout_solver,
            canvas_config: &spread_config,
        };
        run_solver(&request)?
    } else {
        let request = Request {
            request_type: RequestType::SinglePage,
            groups: &[group],
            config: &state.config.book_layout_solver,
            ga_config: &state.config.page_layout_solver,
            canvas_config: &state.config.book,
        };
        run_solver(&request)?
    };

    if result.is_empty() {
        anyhow::bail!("Solver returned no result for page {}", page_idx);
    }

    state.layout[page_idx].slots = result[0].slots.clone();
    state.layout[page_idx].photos = result[0].photos.clone();

    Ok(())
}

/// Wrapper that presents the full cover spread (front+back+spine) as page_width_mm.
struct CoverSpreadConfig<'a> {
    cover: &'a crate::dto_models::CoverConfig,
    inner_page_count: usize,
}

impl crate::dto_models::CanvasConfig for CoverSpreadConfig<'_> {
    fn page_width_mm(&self) -> f64 {
        self.cover.spread_width_mm(self.inner_page_count)
    }
    fn page_height_mm(&self) -> f64 {
        self.cover.height_mm
    }
    fn bleed_mm(&self) -> f64 {
        self.cover.bleed_mm
    }
    fn margin_mm(&self) -> f64 {
        self.cover.margin_mm
    }
    fn gap_mm(&self) -> f64 {
        self.cover.gap_mm
    }
    fn bleed_threshold_mm(&self) -> f64 {
        self.cover.bleed_threshold_mm
    }
}
