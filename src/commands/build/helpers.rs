use anyhow::Result;
use std::sync::atomic::AtomicUsize;
use std::{collections::HashMap, path::PathBuf};
use tracing::{info, warn};

use super::DpiWarning;
use crate::cache::{final_cache, preview_cache};
use crate::dto_models::{PhotoFile, PhotoGroup, ProjectState, build_photo_index};
use crate::output::typst;
use crate::state_manager::StateManager;

pub enum CommitMode {
    Auto,
    Always,
}

/// Data captured from `StateManager` before it is consumed by `finish`/`finish_always`.
pub struct RenderContext {
    pub project_root: PathBuf,
    pub project_name: String,
    pub bleed_mm: f64,
}

impl RenderContext {
    pub fn capture(mgr: &StateManager) -> Self {
        Self {
            project_root: mgr.project_root().to_owned(),
            project_name: mgr.project_name().to_string(),
            bleed_mm: mgr.state.config.book.bleed_mm,
        }
    }
}

pub fn refresh_preview_cache(
    mgr: &mut StateManager,
) -> Result<preview_cache::PreviewCacheResult, anyhow::Error> {
    let preview_cache_dir = mgr.preview_cache_dir();
    let cache_result = preview_cache::ensure_previews(&mut mgr.state, &preview_cache_dir)?;
    if cache_result.created > 0 {
        info!(
            "Preview cache: {} created, {} skipped",
            cache_result.created, cache_result.skipped
        );
    };
    Ok(cache_result)
}

/// Outcome of a cache refresh: how many images were (re)generated and any DPI
/// warnings raised while building the final cache.
pub struct CacheRefresh {
    pub images_processed: usize,
    pub dpi_warnings: Vec<DpiWarning>,
}

impl CacheRefresh {
    /// A refresh that only (re)generated images and cannot carry DPI warnings
    /// (preview builds and skipped refreshes).
    pub fn images_only(images_processed: usize) -> Self {
        Self {
            images_processed,
            dpi_warnings: Vec::new(),
        }
    }
}

/// Builds the final high-resolution image cache for a release build and logs DPI
/// warnings.
pub fn refresh_final_cache(mgr: &mut StateManager) -> Result<CacheRefresh> {
    let dpi = mgr.state.config.book.dpi;
    info!("Release build: generating final PDF at {:.0} DPI...", dpi);

    let progress = AtomicUsize::new(0);
    let final_cache_dir = mgr.final_cache_dir();
    let result = final_cache::build_final_cache(&mut mgr.state, &final_cache_dir, &progress)?;

    info!(
        "Final cache: {} images generated, {} DPI warnings",
        result.created,
        result.dpi_warnings.len()
    );
    log_dpi_warnings(dpi, &result.dpi_warnings);

    Ok(CacheRefresh {
        images_processed: result.created,
        dpi_warnings: result.dpi_warnings,
    })
}

/// Logs each photo that will be rendered below the target DPI.
fn log_dpi_warnings(target_dpi: f64, warnings: &[DpiWarning]) {
    if warnings.is_empty() {
        return;
    }
    warn!(
        "\nWARNING: Some photos will be displayed below {:.0} DPI:",
        target_dpi
    );
    for w in warnings {
        warn!(
            "  Page {}: {} - {:.2} DPI ({}x{} px in {:.1}x{:.1} mm slot)",
            w.page,
            w.photo_id,
            w.actual_dpi,
            w.original_px.0,
            w.original_px.1,
            w.slot_mm.0,
            w.slot_mm.1
        );
    }
}

pub enum PdfTarget {
    Preview,
    Final,
}

/// Renders the PDF or returns only the output path when `skip_pdf` is true.
pub fn render_pdf(ctx: &RenderContext, target: PdfTarget, skip_pdf: bool) -> Result<PathBuf> {
    let root = &ctx.project_root;
    let name = &ctx.project_name;
    let bleed = ctx.bleed_mm;
    if skip_pdf {
        return Ok(root.join(format!("{name}.pdf")));
    }
    match target {
        PdfTarget::Preview => {
            let path = typst::compile_preview(root, name, bleed)?;
            info!("PDF updated: {}", path.display());
            Ok(path)
        }
        PdfTarget::Final => {
            let path = typst::compile_final(root, name, bleed)?;
            info!("Final PDF generated: {}", path.display());
            Ok(path)
        }
    }
}

/// Sammelt alle Fotos aus dem Seitenbereich und rekonstruiert PhotoGroups.
///
/// # Arguments
/// * `start` - **0-based** index (inclusive), e.g., 0 for the first page
/// * `end` - Slice end (exclusive), e.g., 2 to include pages at indices 0 and 1
///
/// # Example
/// To get photos from user pages 1-2 (1-based): call with `start = 0, end = 2`
pub fn collect_photos_as_groups(state: &ProjectState, start: usize, end: usize) -> Vec<PhotoGroup> {
    let photo_index = build_photo_index(&state.photos);

    // Photo-IDs aus dem Bereich sammeln
    let page_photo_ids: Vec<&str> = state.layout[start..end]
        .iter()
        .flat_map(|p| p.photos.iter().map(String::as_str))
        .collect();

    // Nach Originalgruppe aufteilen
    let mut groups_map: HashMap<&str, Vec<PhotoFile>> = HashMap::new();
    for id in &page_photo_ids {
        if let Some((pf, group_name)) = photo_index.get(*id) {
            groups_map
                .entry(group_name)
                .or_default()
                .push((*pf).clone());
        }
    }

    // sort_key aus state.photos übernehmen
    let group_sort_keys: HashMap<&str, &str> = state
        .photos
        .iter()
        .map(|g| (g.group.as_str(), g.sort_key.as_str()))
        .collect();

    let mut groups: Vec<PhotoGroup> = groups_map
        .into_iter()
        .map(|(name, files)| PhotoGroup {
            group: name.to_string(),
            sort_key: group_sort_keys.get(name).unwrap_or(&"").to_string(),
            files,
        })
        .collect();

    groups.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));
    groups
}
