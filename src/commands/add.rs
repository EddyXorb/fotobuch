//! `fotobuch add` command - Add photos to the project

use crate::input::metadata::compute_partial_hash;
use crate::input::scanner::scan_photo_dirs;
use crate::project::git;
use crate::project::state::{PhotoFile, PhotoGroup, ProjectState};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

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
/// 1. Scan directories recursively for image files
/// 2. Group photos by containing directory
/// 3. Read EXIF data (timestamp, dimensions)
/// 4. Determine group timestamp via heuristic (directory name > EXIF > mtime)
/// 5. Check for duplicates (partial hash: first 64KB + last 64KB + size)
/// 6. Update fotobuch.yaml (photos section)
/// 7. Git commit: "add: N photos in M groups"
///
/// # Arguments
/// * `project_root` - Path to the project directory
/// * `config` - Configuration for adding photos
///
/// # Returns
/// * `AddResult` with summary of added groups and warnings
pub fn add(project_root: &Path, config: &AddConfig) -> Result<AddResult> {
    let yaml_path = project_root.join("fotobuch.yaml");
    
    // Load existing project state
    let mut state = ProjectState::load(&yaml_path)
        .context("Failed to load fotobuch.yaml")?;

    // Collect existing hashes and paths for duplicate detection
    let mut existing_hashes: HashSet<String> = HashSet::new();
    let mut existing_paths: HashSet<PathBuf> = HashSet::new();
    
    for group in &state.photos {
        for photo in &group.files {
            existing_paths.insert(PathBuf::from(&photo.source));
        }
    }

    // Scan all provided paths
    let mut all_groups = Vec::new();
    for path in &config.paths {
        let groups = scan_photo_dirs(path)
            .with_context(|| format!("Failed to scan directory: {}", path.display()))?;
        all_groups.extend(groups);
    }

    let mut added_groups = Vec::new();
    let mut skipped_count = 0;
    let mut warnings = Vec::new();

    // Process each group
    for scanned_group in all_groups {
        let mut photo_files = Vec::new();
        
        for scanned_photo in &scanned_group.photos {
            // Skip if path already exists
            if existing_paths.contains(&scanned_photo.path) {
                skipped_count += 1;
                continue;
            }

            // Compute hash
            let hash = match compute_partial_hash(&scanned_photo.path) {
                Ok(h) => h,
                Err(e) => {
                    warnings.push(format!("Failed to hash {}: {}", scanned_photo.path.display(), e));
                    continue;
                }
            };

            // Check for duplicate hash
            if !config.allow_duplicates && existing_hashes.contains(&hash) {
                warnings.push(format!("Skipping duplicate (by hash): {}", scanned_photo.path.display()));
                skipped_count += 1;
                continue;
            }

            // Get dimensions
            let (width_px, height_px) = match scanned_photo.dimensions {
                Some((w, h)) => (w, h),
                None => {
                    warnings.push(format!("Missing dimensions for {}", scanned_photo.path.display()));
                    continue;
                }
            };

            // Generate ID: group/filename
            let filename = scanned_photo.path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown.jpg");
            let id = format!("{}/{}", scanned_group.label, filename);

            // Convert to PhotoFile
            let photo_file = PhotoFile {
                id: id.clone(),
                source: scanned_photo.path.to_string_lossy().to_string(),
                width_px,
                height_px,
                area_weight: 1.0,
                hash: Some(hash.clone()),
            };

            photo_files.push(photo_file);
            existing_hashes.insert(hash);
            existing_paths.insert(scanned_photo.path.clone());
        }

        // Skip empty groups
        if photo_files.is_empty() {
            continue;
        }

        // Convert timestamp to ISO 8601
        let sort_key = scanned_group.timestamp
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
            .unwrap_or_else(|| "9999-12-31T23:59:59".to_string()); // Groups without timestamp go last

        let photo_count = photo_files.len();
        
        // Create PhotoGroup
        let photo_group = PhotoGroup {
            group: scanned_group.label.clone(),
            sort_key: sort_key.clone(),
            files: photo_files,
        };

        state.photos.push(photo_group);

        added_groups.push(GroupSummary {
            name: scanned_group.label,
            photo_count,
            timestamp: sort_key,
        });
    }

    // Sort groups by timestamp
    state.photos.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));

    // Save updated state
    state.save(&yaml_path)
        .context("Failed to save fotobuch.yaml")?;

    // Git commit if in a git repository
    if git::is_git_repo(project_root) {
        let total_photos: usize = added_groups.iter().map(|g| g.photo_count).sum();
        let commit_msg = format!("add: {} photos in {} groups", total_photos, added_groups.len());
        
        if let Err(e) = git::commit(project_root, &commit_msg) {
            warnings.push(format!("Git commit failed: {}", e));
        }
    }

    Ok(AddResult {
        groups_added: added_groups,
        skipped: skipped_count,
        warnings,
    })
}
