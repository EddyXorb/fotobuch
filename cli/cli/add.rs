//! Handler for `fotobuch add` command

use anyhow::{Context, Result};
use fotobuch::commands;
use regex::Regex;
use std::path::PathBuf;
use tracing::{info, warn};

pub struct AddArgs {
    pub paths: Vec<PathBuf>,
    pub allow_duplicates: bool,
    pub filter_xmp: Vec<String>,
    pub filter: Vec<String>,
    pub dry: bool,
    pub update: bool,
    pub recursive: bool,
    pub weight: f64,
}

pub fn handle(args: AddArgs) -> Result<()> {
    let project_root = std::env::current_dir().context("Failed to determine current directory")?;

    let xmp_filters: Result<Vec<Regex>> = args
        .filter_xmp
        .iter()
        .map(|pat| Regex::new(pat).with_context(|| format!("Invalid --filter-xmp regex: {pat}")))
        .collect();
    let xmp_filters = xmp_filters?;

    let source_filters: Result<Vec<Regex>> = args
        .filter
        .iter()
        .map(|pat| Regex::new(pat).with_context(|| format!("Invalid --filter regex: {pat}")))
        .collect();
    let source_filters = source_filters?;

    let config = commands::AddConfig {
        paths: args.paths,
        allow_duplicates: args.allow_duplicates,
        xmp_filters,
        source_filters,
        dry_run: args.dry,
        update: args.update,
        recursive: args.recursive,
        weight: args.weight,
    };

    let result = commands::add(&project_root, &config)?;

    if result.dry_run {
        print_dry_run(&result);
    } else {
        print_result(&result);
    }

    Ok(())
}

fn print_result(result: &commands::AddResult) {
    if result.groups_added.is_empty() {
        info!("ℹ️  No new photos added (all skipped).");
    } else {
        info!(
            "✅ Added {} photos in {} groups",
            result
                .groups_added
                .iter()
                .map(|g| g.photo_count)
                .sum::<usize>(),
            result.groups_added.len()
        );
        for group in &result.groups_added {
            info!(
                "   📁 {} — {} photos ({})",
                group.name,
                group.photo_count,
                &group.timestamp[..16] // show only up to minutes for brevity
            );
        }
    }

    print_shared_stats(result);
}

fn print_dry_run(result: &commands::AddResult) {
    info!("🔍 Dry run — no changes written.\n");

    if result.groups_added.is_empty() {
        info!("ℹ️  No photos would be added.");
    } else {
        info!("Would add:");
        for group in &result.groups_added {
            info!(
                "   📁 {} — {} photos ({})",
                group.name, group.photo_count, group.timestamp
            );
        }
        info!(
            "... {} photos in {} groups.",
            result
                .groups_added
                .iter()
                .map(|g| g.photo_count)
                .sum::<usize>(),
            result.groups_added.len()
        );
    }

    print_shared_stats(result);
}

fn print_shared_stats(result: &commands::AddResult) {
    if result.xmp_filtered > 0 {
        info!("🔎 Filtered {} photos by XMP metadata", result.xmp_filtered);
    }
    if result.source_filtered > 0 {
        info!(
            "🔎 Filtered {} photos by source path pattern",
            result.source_filtered
        );
    }
    if result.updated > 0 {
        info!("🔄 Updated {} changed photos", result.updated);
    }
    if result.skipped > 0 {
        info!("⏭️  Skipped {} duplicate photos", result.skipped);
    }
    if !result.warnings.is_empty() {
        warn!("⚠️  Warnings:");
        for warn in &result.warnings {
            warn!("   - {}", warn);
        }
    }
}
