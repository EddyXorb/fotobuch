//! Handler for `fotobuch remove` command

use anyhow::Context;
use anyhow::Result;
use photobook_solver::commands;
use tracing::info;

pub fn handle(patterns: Vec<String>, keep_files: bool, unplaced: bool) -> Result<()> {
    let project_root = std::env::current_dir().context("Failed to determine current directory")?;

    let config = commands::remove::RemoveConfig {
        patterns,
        keep_files,
        unplaced,
    };

    let result = commands::remove(&project_root, &config)?;

    if result.photos_removed == 0 && result.placements_removed == 0 {
        if unplaced {
            info!("ℹ️  No unplaced photos found.");
        } else {
            info!("ℹ️  No photos matched the pattern(s).");
        }
    } else {
        if result.photos_removed > 0 {
            info!("✅ Removed {} photo(s) from project", result.photos_removed);
            if !result.groups_removed.is_empty() {
                info!("   Removed groups: {}", result.groups_removed.join(", "));
            }
        }
        if result.placements_removed > 0 {
            let pages_str = if result.pages_affected.len() == 1 {
                format!("page {}", result.pages_affected[0])
            } else {
                format!("pages {:?}", result.pages_affected)
            };
            info!(
                "✅ Removed {} placement(s) from {}",
                result.placements_removed, pages_str
            );
        }
        if keep_files {
            info!("ℹ️  Photos kept in project as unplaced.");
        }
        info!("🔄 Run 'fotobuch build' or 'fotobuch rebuild' to regenerate PDFs.");
    }

    Ok(())
}
