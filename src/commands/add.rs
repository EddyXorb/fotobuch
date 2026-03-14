//! `fotobuch add` command - Add photos to the project
//!
//! This module provides the main add command logic and supporting structures.
//! Helper functions are organized in submodules:
//! - `deduplication`: Duplicate detection via path and hash comparison
//! - `merge`: Group merging logic for combining photo groups

mod deduplication;
mod merge;

pub use deduplication::deduplicate;
pub use merge::merge_group;

use anyhow::{Context, Result};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::input::scanner;
use crate::input::xmp;
use crate::state_manager::StateManager;

/// Configuration for adding photos
#[derive(Debug)]
pub struct AddConfig {
    /// Directories or individual files to add
    pub paths: Vec<PathBuf>,
    /// Allow adding files with identical content (hash collision)
    pub allow_duplicates: bool,
    /// When set, only include photos whose XMP metadata matches this regex
    pub xmp_filter: Option<Regex>,
    /// Preview mode: scan and report what would be added without touching the project
    pub dry_run: bool,
    /// Re-add photos whose path already exists but whose content (hash) has changed
    pub update: bool,
}

/// Summary of a single added (or would-be-added) group
#[derive(Debug)]
pub struct GroupSummary {
    /// Group name (relative path from add argument)
    pub name: String,
    /// Number of photos in this group
    pub photo_count: usize,
    /// Timestamp determined for this group (ISO 8601)
    pub timestamp: String,
}

/// Result of adding photos
#[derive(Debug)]
pub struct AddResult {
    /// Groups that were added (or would be added in dry-run mode)
    pub groups_added: Vec<GroupSummary>,
    /// Number of photos that were skipped (already exist)
    pub skipped: usize,
    /// Number of photos that were excluded by the XMP filter
    pub xmp_filtered: usize,
    /// Warnings about duplicates or other issues
    pub warnings: Vec<String>,
    /// Whether this was a dry run (no changes written)
    pub dry_run: bool,
    /// Number of photos whose content changed and were updated
    pub updated: usize,
}

/// Add photos to the project (or preview what would be added with `dry_run`).
///
/// # Steps
/// 1. Open StateManager (commits any manual edits) — skipped in dry-run
/// 2. Scan directories for photo files
/// 3. Apply XMP filter (if configured)
/// 4. Deduplicate (path and hash check)
/// 5. Merge groups (extend existing or add new) — skipped in dry-run
/// 6. Sort groups by sort_key — skipped in dry-run
/// 7. Commit changes via StateManager — skipped in dry-run
pub fn add(project_root: &Path, config: &AddConfig) -> Result<AddResult> {
    let mut mgr =
        StateManager::open(project_root).context("Failed to open project via StateManager")?;

    let all_files: Vec<_> = mgr
        .state
        .photos
        .iter()
        .flat_map(|g| g.files.iter())
        .collect();

    let mut existing_paths: HashSet<PathBuf> = all_files
        .iter()
        .map(|f| PathBuf::from(&f.source))
        .collect();

    let mut existing_hashes: HashSet<String> = all_files
        .iter()
        .filter(|f| !f.hash.is_empty())
        .map(|f| f.hash.clone())
        .collect();

    let existing_path_hashes: HashMap<PathBuf, String> = all_files
        .iter()
        .filter(|f| !f.hash.is_empty())
        .map(|f| (PathBuf::from(&f.source), f.hash.clone()))
        .collect();

    let mut all_warnings = Vec::new();
    let mut total_skipped = 0;
    let mut total_updated = 0;
    let mut total_xmp_filtered = 0;
    let mut groups_added = Vec::new();

    for path in &config.paths {
        let scanned_groups = if path.is_file() {
            scanner::scan_single_photo_file(path)
                .with_context(|| format!("Failed to scan file {}", path.display()))?
        } else if path.is_dir() {
            scanner::scan_photo_group_dirs(path)
                .with_context(|| format!("Failed to scan directory {}", path.display()))?
        } else {
            anyhow::bail!("Path is neither a file nor a directory: {}", path.display());
        };

        for mut scanned_group in scanned_groups {
            // Apply XMP filter before dedup (cheap to skip files early)
            if let Some(pattern) = &config.xmp_filter {
                let before = scanned_group.files.len();
                scanned_group
                    .files
                    .retain(|f| xmp::xmp_matches(Path::new(&f.source), pattern).unwrap_or(true));
                total_xmp_filtered += before - scanned_group.files.len();
            }

            let (kept_files, updated_files, skipped, warnings) = deduplicate(
                &mut scanned_group.files,
                &existing_paths,
                &existing_hashes,
                config.allow_duplicates,
                config.update,
                &existing_path_hashes,
            );

            total_skipped += skipped;
            total_updated += updated_files.len();
            all_warnings.extend(warnings);

            if !config.dry_run {
                // Apply in-place updates to existing photos
                for updated_file in &updated_files {
                    let source = &updated_file.source;
                    for group in &mut mgr.state.photos {
                        if let Some(existing) = group.files.iter_mut().find(|f| f.source == *source) {
                            *existing = updated_file.clone();
                            break;
                        }
                    }
                }
            }

            if kept_files.is_empty() {
                continue;
            }

            // Track newly seen paths/hashes to catch cross-group duplicates
            for file in &kept_files {
                existing_paths.insert(PathBuf::from(&file.source));
                if !file.hash.is_empty() {
                    existing_hashes.insert(file.hash.clone());
                }
            }

            let group_name = scanned_group.group.clone();
            let photo_count = kept_files.len();
            let timestamp = scanned_group.sort_key.clone();

            if !config.dry_run {
                scanned_group.files = kept_files;
                merge_group(&mut mgr.state.photos, scanned_group);
            }

            groups_added.push(GroupSummary {
                name: group_name,
                photo_count,
                timestamp,
            });
        }
    }

    if !config.dry_run {
        mgr.state.photos.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));

        let total_photos: usize = groups_added.iter().map(|g| g.photo_count).sum();
        mgr.finish(&format!(
            "add: {} photos in {} groups",
            total_photos,
            groups_added.len()
        ))?;
    }

    Ok(AddResult {
        groups_added,
        skipped: total_skipped,
        xmp_filtered: total_xmp_filtered,
        warnings: all_warnings,
        dry_run: config.dry_run,
        updated: total_updated,
    })
}
