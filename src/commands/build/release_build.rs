use std::{path::Path, sync::atomic::AtomicUsize};

use crate::commands::BuildResult;
use crate::{cache::final_cache, state_manager::StateManager};

use crate::output::typst;
use anyhow::Result;
use tracing::{info, warn};
/// Performs release build: generates final high-quality PDF at 300 DPI.
///
/// # Requirements
/// - Layout must be clean (no uncommitted changes)
/// - All photos must be available
///
/// # Steps
/// 1. Verify layout is clean
/// 2. Generate final cache (300 DPI) and collect DPI warnings
/// 3. Compile final.typ -> final.pdf
/// 4. Save and commit
pub fn release_build(mgr: StateManager, project_root: &Path) -> Result<super::BuildResult> {
    info!("Release build: generating final PDF at 300 DPI...");

    // 1. Check that layout is clean (no changes since last build)
    if mgr.has_changes_since_last_build() {
        anyhow::bail!(
            "Layout has changes since last build. Run `fotobuch build` first to commit all changes."
        );
    }

    if mgr.state.layout.is_empty() {
        anyhow::bail!("No layout found. Run `fotobuch build` first to generate layout.");
    }

    // 2. Generate final cache at 300 DPI
    let progress = AtomicUsize::new(0);
    let final_cache_dir = mgr.final_cache_dir();
    let final_result = final_cache::build_final_cache(&mgr.state, &final_cache_dir, &progress)?;

    info!(
        "Final cache: {} images generated, {} DPI warnings",
        final_result.created,
        final_result.dpi_warnings.len()
    );

    // Print DPI warnings
    if !final_result.dpi_warnings.is_empty() {
        warn!("\nWARNING: Some photos will be displayed below 300 DPI:");
        for warning in &final_result.dpi_warnings {
            warn!(
                "  Page {}: {} - {:.1} DPI ({}x{} px in {:.1}x{:.1} mm slot)",
                warning.page,
                warning.photo_id,
                warning.actual_dpi,
                warning.original_px.0,
                warning.original_px.1,
                warning.slot_mm.0,
                warning.slot_mm.1
            );
        }
        info!("");
    }

    // 3. Compile final.typ -> final.pdf
    let pdf_path = typst::compile_final(project_root, mgr.project_name())?;
    info!("Final PDF generated: {}", pdf_path.display());

    // 4. Save state and commit
    let page_count = mgr.state.layout.len();
    let total_photos: usize = mgr.state.layout.iter().map(|p| p.photos.len()).sum();

    mgr.finish_always(&format!(
        "release: {} pages, {} photos",
        page_count, total_photos
    ))?;

    Ok(BuildResult {
        pdf_path,
        pages_rebuilt: vec![], // Release doesn't rebuild layout
        pages_swapped: vec![],
        images_processed: final_result.created,
        total_cost: 0.0, // Not relevant for release
        dpi_warnings: final_result.dpi_warnings,
        nothing_to_do: false,
    })
}
