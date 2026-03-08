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

use std::path::PathBuf;

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

// TODO: Implement add() function with StateManager
// pub fn add(project_root: &Path, config: &AddConfig) -> Result<AddResult>
