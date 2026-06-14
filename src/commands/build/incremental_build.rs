use super::BuildOptions;
use super::core::rebuild_single_page::rebuild_single_page;
use super::helpers::{
    PdfTarget, RenderContext, build_photo_index, render_pdf, update_preview_cache,
};
use crate::commands::CommandOutput;
use crate::commands::build::BuildResult;
use crate::state_manager::StateManager;
use anyhow::Result;
use std::path::Path;
use tracing::info;

/// Performs incremental build: updates only modified pages.
pub fn incremental_build(
    mut mgr: StateManager,
    project_root: &Path,
    page_filter: Option<&[usize]>,
    opts: BuildOptions,
) -> Result<CommandOutput<BuildResult>> {
    info!("Incremental build: checking for changes...");

    // 1. Generate/update preview cache
    let cache_result = if opts.skip_cache_update {
        Default::default()
    } else {
        update_preview_cache(&mut mgr)?
    };

    // 2. Detect which pages need rebuilding
    let page_indices_needing_rebuild = mgr.outdated_pages_indices();

    // 3. Apply page filter if specified
    let pages_needing_rebuild = apply_page_filter(page_indices_needing_rebuild, page_filter);

    if pages_needing_rebuild.is_empty() {
        info!("No changes detected. Nothing to do. Build only pdf.");
        let pdf_path = render_pdf(
            project_root,
            mgr.project_name(),
            mgr.state.config.book.bleed_mm,
            PdfTarget::Preview,
            opts.skip_pdf,
        )?;

        let changed_state = mgr.finish("")?;
        return Ok(CommandOutput {
            result: BuildResult {
                pdf_path,
                pages_rebuilt: vec![],
                pages_swapped: vec![],
                images_processed: cache_result.created,
                total_cost: 0.0,
                dpi_warnings: vec![],
                nothing_to_do: true,
            },
            changed_state,
        });
    }

    info!(
        "Rebuilding {} page(s): {:?}",
        pages_needing_rebuild.len(),
        pages_needing_rebuild
    );

    // 4. Build photo index for fast lookup
    let photo_index = build_photo_index(&mgr.state.photos);

    // 5. Rebuild each modified page (skip manual pages)
    for &page_idx in &pages_needing_rebuild {
        if mgr.state.layout[page_idx].mode == crate::dto_models::PageMode::Manual {
            continue;
        }
        rebuild_single_page(&mut mgr.state, page_idx, &photo_index)?;
    }

    let ctx = RenderContext::capture(&mgr);
    let total_cost = 0.0; //TODO: calculate actual cost from modified pages when available
    let changed_state = mgr.finish(&format!(
        "build: {} page(s) rebuilt",
        pages_needing_rebuild.len()
    ))?;

    let pdf_path = render_pdf(
        project_root,
        &ctx.project_name,
        ctx.bleed_mm,
        PdfTarget::Preview,
        opts.skip_pdf,
    )?;

    Ok(CommandOutput {
        result: BuildResult {
            pdf_path,
            pages_rebuilt: pages_needing_rebuild,
            pages_swapped: vec![],
            images_processed: cache_result.created,
            total_cost,
            dpi_warnings: vec![],
            nothing_to_do: false,
        },
        changed_state,
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
