use super::super::BuildResult;
use super::rebuild_single_page::rebuild_single_page;
use crate::dto_models::PhotoFile;
use crate::output::typst;
use crate::state_manager::StateManager;
use crate::{cache::preview, dto_models::PhotoGroup};
use anyhow::Result;
use std::collections::HashMap;
use std::{path::Path, sync::atomic::AtomicUsize};

/// Performs incremental build: updates only modified pages.
pub fn incremental_build(
    mut mgr: StateManager,
    project_root: &Path,
    page_filter: Option<&[usize]>,
) -> Result<BuildResult> {
    println!("Incremental build: checking for changes...");

    // 1. Generate/update preview cache
    let progress = AtomicUsize::new(0);
    let preview_cache_dir = mgr.preview_cache_dir();
    let cache_result = preview::ensure_previews(&mgr.state, &preview_cache_dir, &progress)?;

    if cache_result.created > 0 {
        println!(
            "Preview cache: {} created, {} skipped",
            cache_result.created, cache_result.skipped
        );
    }

    // 2. Detect which pages need rebuilding
    let pages_needing_rebuild = mgr.modified_pages();

    // 3. Apply page filter if specified
    let pages_needing_rebuild = apply_page_filter(pages_needing_rebuild, page_filter);

    if pages_needing_rebuild.is_empty() {
        println!("No changes detected. Nothing to do.");
        return Ok(BuildResult {
            pdf_path: project_root.join(format!("{}.pdf", mgr.project_name())),
            pages_rebuilt: vec![],
            pages_swapped: vec![],
            images_processed: cache_result.created,
            total_cost: 0.0,
            dpi_warnings: vec![],
            nothing_to_do: true,
        });
    }

    println!(
        "Rebuilding {} page(s): {:?}",
        pages_needing_rebuild.len(),
        pages_needing_rebuild
    );

    // 4. Build photo index for fast lookup
    let photo_index = build_photo_index(&mgr.state.photos);

    // 5. Rebuild each modified page
    for &page_num in &pages_needing_rebuild {
        rebuild_single_page(&mut mgr.state, page_num, &photo_index)?;
    }

    // 6. Compile Typst template to PDF
    let pdf_path = typst::compile_preview(project_root, mgr.project_name())?;
    println!("PDF updated: {}", pdf_path.display());

    // 7. Save state and commit
    let total_cost = 0.0; //TODO: calculate actual cost from modified pages when available
    mgr.finish(&format!(
        "build: {} page(s) rebuilt",
        pages_needing_rebuild.len()
    ))?;

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

/// Builds a photo index for fast lookup: photo_id -> (PhotoFile, group_name).
fn build_photo_index(photos: &[PhotoGroup]) -> HashMap<String, (PhotoFile, String)> {
    photos
        .iter()
        .flat_map(|group| {
            group
                .files
                .iter()
                .map(move |file| (file.id.clone(), (file.clone(), group.group.clone())))
        })
        .collect()
}
