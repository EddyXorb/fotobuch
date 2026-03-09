//! Handler for `fotobuch remove` command

use anyhow::Context;
use anyhow::Result;
use photobook_solver::commands;

pub fn handle(patterns: Vec<String>, keep_files: bool) -> Result<()> {
    let project_root = std::env::current_dir()
        .context("Failed to determine current directory")?;

    let config = commands::remove::RemoveConfig {
        patterns,
        keep_files,
    };

    let result = commands::remove(&project_root, &config)?;

    if result.photos_removed == 0 && result.placements_removed == 0 {
        println!("ℹ️  No photos matched the pattern(s).");
    } else {
        if result.photos_removed > 0 {
            println!("✅ Removed {} photo(s) from project", result.photos_removed);
            if !result.groups_removed.is_empty() {
                println!("   Removed groups: {}", result.groups_removed.join(", "));
            }
        }
        if result.placements_removed > 0 {
            let pages_str = if result.pages_affected.len() == 1 {
                format!("page {}", result.pages_affected[0])
            } else {
                format!("pages {:?}", result.pages_affected)
            };
            println!("✅ Removed {} placement(s) from {}", result.placements_removed, pages_str);
        }
        if keep_files {
            println!("ℹ️  Photos kept in project as unplaced.");
        }
        println!("🔄 Run 'fotobuch build' or 'fotobuch rebuild' to regenerate PDFs.");
    }

    Ok(())
}
