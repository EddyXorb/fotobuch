use std::{path::Path, sync::atomic::AtomicUsize};

use crate::commands::{BuildResult, CommandOutput};
use crate::{cache::final_cache, state_manager::StateManager};

use crate::output::typst;
use anyhow::Result;
use tracing::{info, warn};
/// Performs release build: generates final high-quality PDF at the configured DPI.
///
/// # Requirements
/// - Layout must be clean (no uncommitted changes), unless force is true
/// - All photos must be available
///
/// # Steps
/// 1. Verify layout is clean (skip if force is true)
/// 2. Generate final cache and collect DPI warnings
/// 3. Compile final.typ -> final.pdf
/// 4. Save and commit
pub fn release_build(
    mut mgr: StateManager,
    project_root: &Path,
    force: bool,
) -> Result<CommandOutput<BuildResult>> {
    let dpi = mgr.state.config.book.dpi;
    info!("Release build: generating final PDF at {:.0} DPI...", dpi);

    // 1. Check that layout is clean (no changes since last build)
    if !force {
        let changed_pages: Vec<_> = mgr
            .outdated_pages_indices()
            .into_iter()
            .filter(|i| {
                if mgr.state.config.book.cover.active {
                    *i != 0
                } else {
                    true
                }
            })
            .collect();
        if !changed_pages.is_empty() {
            anyhow::bail!(
                "Layout has changes since last build. Changed pages: {:?}. Run `fotobuch build` first to commit all changes, or use `fotobuch build release --force`.",
                changed_pages
            );
        }
    }

    if mgr.state.layout.is_empty() {
        anyhow::bail!("No layout found. Run `fotobuch build` first to generate layout.");
    }

    // 2. Generate final cache
    let progress = AtomicUsize::new(0);
    let final_cache_dir = mgr.final_cache_dir();
    let final_result = final_cache::build_final_cache(&mut mgr.state, &final_cache_dir, &progress)?;

    info!(
        "Final cache: {} images generated, {} DPI warnings",
        final_result.created,
        final_result.dpi_warnings.len()
    );

    // Print DPI warnings
    if !final_result.dpi_warnings.is_empty() {
        warn!(
            "\nWARNING: Some photos will be displayed below {:.0} DPI:",
            dpi
        );
        for warning in &final_result.dpi_warnings {
            warn!(
                "  Page {}: {} - {:.2} DPI ({}x{} px in {:.1}x{:.1} mm slot)",
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

    // 4. Save state and commit
    let bleed_mm = mgr.state.config.book.bleed_mm; // need to backup these before mgr gets consumed
    let project_name = mgr.project_name().to_string();

    let page_count = mgr.state.layout.len();
    let total_photos: usize = mgr.state.layout.iter().map(|p| p.photos.len()).sum();

    let changed_state = mgr.finish_always(&format!(
        "release: {} pages, {} photos",
        page_count, total_photos
    ))?;

    // 3. Compile final.typ -> final.pdf (with bleed boxes)
    let pdf_path = typst::compile_final(project_root, &project_name, bleed_mm)?;
    info!("Final PDF generated: {}", pdf_path.display());

    Ok(CommandOutput {
        result: BuildResult {
            pdf_path,
            pages_rebuilt: vec![], // Release doesn't rebuild layout
            pages_swapped: vec![],
            images_processed: final_result.created,
            total_cost: 0.0, // Not relevant for release
            dpi_warnings: final_result.dpi_warnings,
            nothing_to_do: false,
        },
        changed_state,
    })
}
