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
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::input::scanner;
use crate::state_manager::StateManager;

/// Configuration for adding photos
#[derive(Debug, Clone)]
pub struct AddConfig {
    /// Directories or individual files to add
    pub paths: Vec<PathBuf>,
    /// Allow adding files with identical content (hash collision)
    pub allow_duplicates: bool,
}

/// Summary of a single added group
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
    /// Groups that were added
    pub groups_added: Vec<GroupSummary>,
    /// Number of photos that were skipped (already exist)
    pub skipped: usize,
    /// Warnings about duplicates or other issues
    pub warnings: Vec<String>,
}

/// Add photos to the project
///
/// # Steps
/// 1. Open StateManager (commits any manual edits)
/// 2. Scan directories for photo files
/// 3. Deduplicate (path and hash check)
/// 4. Merge groups (extend existing or add new)
/// 5. Sort groups by sort_key
/// 6. Commit changes via StateManager
///
/// # Arguments
/// * `project_root` - Root directory of the fotobuch project (containing .git)
/// * `config` - Configuration (paths to add, duplicate policy)
///
/// # Returns
/// Summary of added groups, skipped photos, and warnings
pub fn add(project_root: &Path, config: &AddConfig) -> Result<AddResult> {
    // Step 1: Open StateManager
    let mut mgr = StateManager::open(project_root)
        .context("Failed to open project via StateManager")?;

    // Step 2: Collect existing paths and hashes
    let mut existing_paths = HashSet::new();
    let mut existing_hashes = HashSet::new();

    for group in &mgr.state.photos {
        for file in &group.files {
            existing_paths.insert(PathBuf::from(&file.source));
            if !file.hash.is_empty() {
                existing_hashes.insert(file.hash.clone());
            }
        }
    }

    // Step 3: Scan directories
    let mut all_warnings = Vec::new();
    let mut total_skipped = 0;
    let mut groups_added = Vec::new();

    for path in &config.paths {
        let scanned_groups = scanner::scan_photo_dirs(path)
            .with_context(|| format!("Failed to scan {}", path.display()))?;

        for mut scanned_group in scanned_groups {
            // Step 4: Deduplicate
            let (kept_files, skipped, warnings) = deduplicate(
                &mut scanned_group.files,
                &existing_paths,
                &existing_hashes,
                config.allow_duplicates,
            );

            total_skipped += skipped;
            all_warnings.extend(warnings);

            // Skip empty groups (all photos were duplicates)
            if kept_files.is_empty() {
                continue;
            }

            // Update existing paths/hashes with newly kept files
            for file in &kept_files {
                existing_paths.insert(PathBuf::from(&file.source));
                if !file.hash.is_empty() {
                    existing_hashes.insert(file.hash.clone());
                }
            }

            // Store summary before merge
            let group_name = scanned_group.group.clone();
            let photo_count = kept_files.len();
            let timestamp = scanned_group.sort_key.clone();

            scanned_group.files = kept_files;

            // Step 5: Merge group
            merge_group(&mut mgr.state.photos, scanned_group);

            groups_added.push(GroupSummary {
                name: group_name,
                photo_count,
                timestamp,
            });
        }
    }

    // Step 6: Sort groups by sort_key
    mgr.state.photos.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));

    // Step 7: Commit via StateManager
    let total_photos: usize = groups_added.iter().map(|g| g.photo_count).sum();
    let commit_msg = format!(
        "add: {} photos in {} groups",
        total_photos,
        groups_added.len()
    );
    mgr.finish(&commit_msg)?;

    Ok(AddResult {
        groups_added,
        skipped: total_skipped,
        warnings: all_warnings,
    })
}
