//! `fotobuch add` command - Add photos to the project

use anyhow::Result;
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
    // TODO: Implement photo adding
    // - Scan directories for image files
    // - Group by containing directory
    // - Read EXIF data
    // - Determine timestamps
    // - Check for duplicates (partial hash)
    // - Update fotobuch.yaml
    // - Git commit

    let _ = (project_root, config); // Silence unused warnings

    Ok(AddResult {
        groups_added: Vec::new(),
        skipped: 0,
        warnings: Vec::new(),
    })
}
