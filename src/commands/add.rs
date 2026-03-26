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

use crate::input::scanner::{self, ScannerInput};
use crate::state_manager::StateManager;

/// Configuration for adding photos
#[derive(Debug)]
pub struct AddConfig {
    /// Directories or individual files to add
    pub paths: Vec<PathBuf>,
    /// Allow adding files with identical content (hash collision)
    pub allow_duplicates: bool,
    /// When set, only include photos whose XMP metadata matches all these regexes
    pub xmp_filters: Vec<Regex>,
    /// When set, only include photos whose source path matches all these regexes
    pub source_filters: Vec<Regex>,
    /// Preview mode: scan and report what would be added without touching the project
    pub dry_run: bool,
    /// Re-add photos whose path already exists but whose content (hash) has changed
    pub update: bool,
    /// Scan directories recursively (each subdir becomes its own group)
    pub recursive: bool,
    /// Area weight for all imported photos (default: 1.0)
    pub weight: f64,
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
    /// Number of photos that were excluded by the source path filter
    pub source_filtered: usize,
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
/// 4. Apply source path filter (if configured)
/// 5. Deduplicate (path and hash check)
/// 6. Merge groups (extend existing or add new) — skipped in dry-run
/// 7. Sort groups by sort_key — skipped in dry-run
/// 8. Commit changes via StateManager — skipped in dry-run
pub fn add(project_root: &Path, config: &AddConfig) -> Result<AddResult> {
    let mut mgr =
        StateManager::open(project_root).context("Failed to open project via StateManager")?;

    let all_files: Vec<_> = mgr
        .state
        .photos
        .iter()
        .flat_map(|g| g.files.iter())
        .collect();

    let mut existing_paths: HashSet<PathBuf> =
        all_files.iter().map(|f| PathBuf::from(&f.source)).collect();

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
    let mut groups_added = Vec::new();

    // Scan photos with filtering
    let scan_output = scanner::scan_photos(ScannerInput {
        paths: config.paths.clone(),
        xmp_filters: config.xmp_filters.clone(),
        source_filters: config.source_filters.clone(),
        recursive: config.recursive,
    })?;

    let total_xmp_filtered = scan_output.stats.xmp_filtered;
    let total_source_filtered = scan_output.stats.source_filtered;

    for mut scanned_group in scan_output.groups {
        for file in &mut scanned_group.files {
            file.area_weight = config.weight;
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
        source_filtered: total_source_filtered,
        warnings: all_warnings,
        dry_run: config.dry_run,
        updated: total_updated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::PhotoFile;
    use chrono::Utc;

    fn make_photo(id: &str, source: &str) -> PhotoFile {
        PhotoFile {
            id: id.to_string(),
            source: source.to_string(),
            width_px: 1920,
            height_px: 1080,
            area_weight: 1.0,
            timestamp: Utc::now(),
            hash: "test".to_string(),
        }
    }

    #[test]
    fn test_source_filter_matches() {
        let filter = Regex::new("vacation").unwrap();
        let files = [
            make_photo("a.jpg", "/photos/vacation/a.jpg"),
            make_photo("b.jpg", "/photos/work/b.jpg"),
            make_photo("c.jpg", "/vacation/c.jpg"),
        ];

        let matched: Vec<_> = files
            .iter()
            .filter(|f| filter.is_match(&f.source))
            .collect();
        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].id, "a.jpg");
        assert_eq!(matched[1].id, "c.jpg");
    }

    #[test]
    fn test_source_filter_with_complex_pattern() {
        let filter = Regex::new(r"\.jpg$").unwrap();
        let files = [
            make_photo("a.jpg", "/photos/a.jpg"),
            make_photo("b.png", "/photos/b.png"),
            make_photo("c.jpg", "/photos/c.jpg"),
        ];

        let matched: Vec<_> = files
            .iter()
            .filter(|f| filter.is_match(&f.source))
            .collect();
        assert_eq!(matched.len(), 2);
    }

    #[test]
    fn test_source_filter_case_insensitive() {
        let filter = Regex::new("(?i)vacation").unwrap();
        let files = [
            make_photo("a.jpg", "/photos/Vacation/a.jpg"),
            make_photo("b.jpg", "/photos/VACATION/b.jpg"),
            make_photo("c.jpg", "/photos/work/c.jpg"),
        ];

        let matched: Vec<_> = files
            .iter()
            .filter(|f| filter.is_match(&f.source))
            .collect();
        assert_eq!(matched.len(), 2);
    }
}
