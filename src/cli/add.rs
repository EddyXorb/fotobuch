//! Handler for `fotobuch add` command

use anyhow::Context;
use anyhow::Result;
use photobook_solver::commands;
use std::path::PathBuf;

pub fn handle(paths: Vec<PathBuf>, allow_duplicates: bool) -> Result<()> {
    let project_root = std::env::current_dir()
        .context("Failed to determine current directory")?;

    let config = commands::AddConfig {
        paths,
        allow_duplicates,
    };

    let result = commands::add(&project_root, &config)?;

    if result.groups_added.is_empty() {
        println!("ℹ️  No new photos added (all skipped).");
    } else {
        println!("✅ Added {} photos in {} groups", 
            result.groups_added.iter().map(|g| g.photo_count).sum::<usize>(),
            result.groups_added.len()
        );
        
        for group in &result.groups_added {
            println!("   📁 {} — {} photos ({})", 
                group.name, 
                group.photo_count, 
                group.timestamp
            );
        }
    }

    if result.skipped > 0 {
        println!("⏭️  Skipped {} duplicate photos", result.skipped);
    }

    if !result.warnings.is_empty() {
        println!("⚠️  Warnings:");
        for warning in &result.warnings {
            println!("   - {}", warning);
        }
    }

    Ok(())
}
