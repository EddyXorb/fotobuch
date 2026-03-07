//! `fotobuch history` command - Show project change history

use anyhow::Result;
use std::path::Path;

/// Single history entry
#[derive(Debug)]
pub struct HistoryEntry {
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Commit message
    pub message: String,
}

/// Show project change history
///
/// This is a thin wrapper around `git log` in the project directory.
/// Shows date + commit message without hash.
///
/// # Steps
/// 1. Run `git log --oneline --format="%ai %s"` in project_root
/// 2. Parse output into HistoryEntry structs
/// 3. Return list for CLI formatting
///
/// For more detailed analysis, users can use git directly:
/// - `git log`
/// - `git diff HEAD~2 HEAD`
/// - `git checkout <hash> -- fotobuch.yaml`
///
/// # Arguments
/// * `project_root` - Path to the project directory
///
/// # Returns
/// * Vector of `HistoryEntry` with timestamp and message
pub fn history(project_root: &Path) -> Result<Vec<HistoryEntry>> {
    // TODO: Implement history command
    // - Run `git log --format="%ai %s"` via std::process::Command
    // - Parse output lines
    // - Split into timestamp and message
    // - Return as Vec<HistoryEntry>
    //
    // If git is not available or not a git repo: return empty vec or error

    let _ = project_root; // Silence unused warning

    Ok(Vec::new())
}
