//! `fotobuch remove` command - Remove photos or groups from the project

use anyhow::Result;
use std::path::Path;

/// Configuration for removing photos
#[derive(Debug, Clone)]
pub struct RemoveConfig {
    /// Photo paths, group names, or glob patterns
    pub patterns: Vec<String>,
    /// Only remove from layout, keep in photos (makes them unplaced)
    pub keep_files: bool,
}

/// Result of removing photos
#[derive(Debug)]
pub struct RemoveResult {
    /// Number of photos removed from photos section
    pub photos_removed: usize,
    /// Number of placements removed from layout
    pub placements_removed: usize,
    /// Groups that were completely removed
    pub groups_removed: Vec<String>,
    /// Pages affected by removals (need rebuild)
    pub pages_affected: Vec<usize>,
}

/// Remove photos or groups from the project
///
/// # Steps
/// 1. Parse patterns (photo paths, group names, glob patterns)
/// 2. Match against photos in fotobuch.yaml
/// 3. If keep_files: remove only from layout, adjust slot indices
/// 4. If not keep_files: remove from photos AND layout, adjust slot indices
/// 5. Update fotobuch.yaml
/// 6. Git commit: "remove: N photos from M groups"
///
/// # Arguments
/// * `project_root` - Path to the project directory
/// * `config` - Configuration for removing photos
///
/// # Returns
/// * `RemoveResult` with summary of removed photos and affected pages
pub fn remove(project_root: &Path, config: &RemoveConfig) -> Result<RemoveResult> {
    // TODO: Implement photo removal
    // - Parse patterns (paths, groups, globs)
    // - Match against photos
    // - Remove from layout (always)
    // - Remove from photos (if not keep_files)
    // - Adjust slot indices
    // - Update fotobuch.yaml
    // - Git commit

    let _ = (project_root, config); // Silence unused warnings

    Ok(RemoveResult {
        photos_removed: 0,
        placements_removed: 0,
        groups_removed: Vec::new(),
        pages_affected: Vec::new(),
    })
}
