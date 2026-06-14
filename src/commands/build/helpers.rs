use anyhow::Result;
use std::{collections::HashMap, path::PathBuf};
use tracing::info;

use crate::cache::preview;
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

pub fn update_preview_cache(
    mgr: &mut StateManager,
) -> Result<preview::PreviewCacheResult, anyhow::Error> {
    let preview_cache_dir = mgr.preview_cache_dir();
    let cache_result = preview::ensure_previews(&mut mgr.state, &preview_cache_dir)?;
    if cache_result.created > 0 {
        info!(
            "Preview cache: {} created, {} skipped",
            cache_result.created, cache_result.skipped
        );
    };
    Ok(cache_result)
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
