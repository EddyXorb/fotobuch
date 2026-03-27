use crate::{
    dto_models::{PhotoFile, PhotoGroup},
    run_solver,
    solver::{
        Request, RequestType,
        cover_solver::{compute_cover_slots, warn_slot_count_mismatch},
    },
};
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
        rebuild_cover_page(state, files, photo_index)
    } else {
        rebuild_inner_page(state, page_idx, files)
    }
}

// ── cover page (index 0) ─────────────────────────────────────────────────────

fn rebuild_cover_page(
    state: &mut crate::dto_models::ProjectState,
    files: Vec<PhotoFile>,
    photo_index: &HashMap<String, (PhotoFile, String)>,
) -> Result<()> {
    let cover = &state.config.book.cover;

    if cover.mode.is_free() {
        // GA solver with correct cover spread dimensions
        rebuild_cover_free(state, files)
    } else {
        // Deterministic cover solver
        rebuild_cover_structured(state, files, photo_index)
    }
}

fn rebuild_cover_free(
    state: &mut crate::dto_models::ProjectState,
    files: Vec<PhotoFile>,
) -> Result<()> {
    let cover = &state.config.book.cover;
    let inner_page_count = state.layout.len() - 1;
    let spread_config = CoverCanvasConfig {
        cover,
        inner_page_count,
    };
    let group = photo_group_for_page(0, files);
    let request = Request {
        request_type: RequestType::SinglePage,
        groups: &[group],
        config: &state.config.book_layout_solver,
        ga_config: &state.config.page_layout_solver,
        canvas_config: &spread_config,
    };
    let result = run_solver(&request)?;
    apply_result(state, 0, result)
}

fn rebuild_cover_structured(
    state: &mut crate::dto_models::ProjectState,
    files: Vec<PhotoFile>,
    photo_index: &HashMap<String, (PhotoFile, String)>,
) -> Result<()> {
    let cover = &state.config.book.cover;
    let mode = cover.mode;
    let inner_page_count = state.layout.len() - 1;

    warn_slot_count_mismatch(mode, files.len());

    let ratios: Vec<f64> = state.layout[0]
        .photos
        .iter()
        .filter_map(|id| photo_index.get(id))
        .map(|(f, _)| f.aspect_ratio())
        .collect();

    let slots = compute_cover_slots(cover, &ratios, inner_page_count)?;

    state.layout[0].slots = slots;
    // photos order is unchanged — cover solver respects the existing assignment
    Ok(())
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

// ── CoverSpreadConfig ─────────────────────────────────────────────────────────

/// Presents the full cover spread (front + back + spine) as `page_width_mm` to the GA solver.
struct CoverCanvasConfig<'a> {
    cover: &'a crate::dto_models::CoverConfig,
    inner_page_count: usize,
}

impl crate::dto_models::CanvasConfig for CoverCanvasConfig<'_> {
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
