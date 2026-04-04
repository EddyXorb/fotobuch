//! Handler for `fotobuch place` command

use anyhow::Context;
use anyhow::Result;
use fotobuch::commands;
use tracing::info;

pub fn handle(filters: Vec<String>, into: Option<usize>) -> Result<()> {
    let project_root = std::env::current_dir().context("Failed to determine current directory")?;

    let config = commands::place::PlaceConfig {
        filters,
        into_page: into,
    };

    let output = commands::place::place(&project_root, &config)?;

    if output.result.photos_placed == 0 {
        info!("ℹ️  No photos to place.");
    } else {
        let pages_str = if output.result.pages_affected.len() == 1 {
            format!("page {}", output.result.pages_affected[0])
        } else {
            format!("pages {:?}", output.result.pages_affected)
        };
        info!(
            "✅ Placed {} photo(s) onto {}",
            output.result.photos_placed, pages_str
        );
        info!("🔄 Run 'fotobuch build' or 'fotobuch rebuild' to regenerate PDFs.");
    }

    Ok(())
}
