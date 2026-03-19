use super::super::BuildResult;
use super::helpers::{build_photo_index, update_preview_pdf};
use super::rebuild_single_page::rebuild_single_page;
use crate::cache::preview;
use crate::state_manager::StateManager;
use anyhow::Result;
use std::path::Path;
use tracing::{info, warn};

/// Performs incremental build: updates only modified pages.
pub fn incremental_build(
    mut mgr: StateManager,
    project_root: &Path,
    page_filter: Option<&[usize]>,
) -> Result<BuildResult> {
    info!("Incremental build: checking for changes...");

    // 1. Generate/update preview cache
    let preview_cache_dir = mgr.preview_cache_dir();
    let cache_result = preview::ensure_previews(&mgr.state, &preview_cache_dir)?;

    if cache_result.created > 0 {
        info!(
            "Preview cache: {} created, {} skipped",
            cache_result.created, cache_result.skipped
        );
    }

    // 2. Detect which pages need rebuilding
    let mut page_indices_needing_rebuild = mgr.outdated_pages_indices();

    // 3. If cover is active and index 0 is outdated, skip it and warn the user
    let has_cover = mgr.state.config.book.cover.as_ref().is_some_and(|c| c.active);
    if has_cover && page_indices_needing_rebuild.contains(&0) {
        warn!(
            "Cover page (index 0) has changes but will not be rebuilt automatically. \
             Use `rebuild --page 0` to rebuild it explicitly."
        );
        page_indices_needing_rebuild.retain(|&idx| idx != 0);
    }

    // 4. Apply page filter if specified
    let pages_needing_rebuild = apply_page_filter(page_indices_needing_rebuild, page_filter);

    if pages_needing_rebuild.is_empty() {
        info!("No changes detected. Nothing to do. Build only pdf.");
        let pdf_path = update_preview_pdf(
            project_root,
            mgr.state.config.book.bleed_mm,
            mgr.project_name(),
        )?;

        return Ok(BuildResult {
            pdf_path,
            pages_rebuilt: vec![],
            pages_swapped: vec![],
            images_processed: cache_result.created,
            total_cost: 0.0,
            dpi_warnings: vec![],
            nothing_to_do: true,
        });
    }

    info!(
        "Rebuilding {} page(s): {:?}",
        pages_needing_rebuild.len(),
        pages_needing_rebuild
    );

    // 4. Build photo index for fast lookup
    let photo_index = build_photo_index(&mgr.state.photos);

    // 5. Rebuild each modified page
    for &page_idx in &pages_needing_rebuild {
        rebuild_single_page(&mut mgr.state, page_idx, &photo_index)?;
    }

    // 7. Save state and commit
    let project_name = mgr.project_name().to_string(); // need to backup these before mgr gets consumed
    let bleed_mm = mgr.state.config.book.bleed_mm;
    let total_cost = 0.0; //TODO: calculate actual cost from modified pages when available
    mgr.finish(&format!(
        "build: {} page(s) rebuilt",
        pages_needing_rebuild.len()
    ))?;

    // 6. Compile Typst template to PDF
    let pdf_path = update_preview_pdf(project_root, bleed_mm, &project_name)?;

    Ok(BuildResult {
        pdf_path,
        pages_rebuilt: pages_needing_rebuild,
        pages_swapped: vec![],
        images_processed: cache_result.created,
        total_cost,
        dpi_warnings: vec![],
        nothing_to_do: false,
    })
}

/// Applies page filter to the list of pages needing rebuild.
/// If filter is None, returns all pages. Otherwise returns only pages in the filter.
fn apply_page_filter(mut pages: Vec<usize>, filter: Option<&[usize]>) -> Vec<usize> {
    if let Some(filter_pages) = filter {
        pages.retain(|p| filter_pages.contains(p));
    }
    pages
}
