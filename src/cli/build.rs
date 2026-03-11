//! Handler for `fotobuch build` command

use anyhow::Context;
use anyhow::Result;
use photobook_solver::commands;
use tracing::info;
use tracing::warn;

pub fn handle(release: bool, pages: Option<Vec<usize>>) -> Result<()> {
    let project_root = std::env::current_dir().context("Failed to determine current directory")?;

    let config = commands::build::BuildConfig { release, pages };

    let result = commands::build::build(&project_root, &config)?;

    if result.nothing_to_do {
        info!("Nothing to do.");
        return Ok(());
    }

    if !result.pages_rebuilt.is_empty() {
        info!(
            "Rebuilt {} page(s): {:?}",
            result.pages_rebuilt.len(),
            result.pages_rebuilt
        );
    }
    info!("PDF: {}", result.pdf_path.display());

    if !result.dpi_warnings.is_empty() {
        warn!(
            "\nWARNING: {} photo(s) below 300 DPI:",
            result.dpi_warnings.len()
        );
        for w in &result.dpi_warnings {
            warn!(
                "  Page {}: {} — {:.0} DPI",
                w.page, w.photo_id, w.actual_dpi
            );
        }
    }

    Ok(())
}
