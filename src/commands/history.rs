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
/// # Arguments
/// * `project_root` - Path to the project directory
///
/// # Returns
/// * Vector of `HistoryEntry` with timestamp and message
/// * Empty vector if no git repo or no commits
pub fn history(project_root: &Path) -> Result<Vec<HistoryEntry>> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["log", "--format=%ai\t%s"])
        .current_dir(project_root)
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new()); // Kein Git oder keine Commits
    }

    let entries = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (ts, msg) = line.split_once('\t')?;
            Some(HistoryEntry {
                timestamp: ts.trim().to_string(),
                message: msg.to_string(),
            })
        })
        .collect();

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_entry_creation() {
        let entry = HistoryEntry {
            timestamp: "2024-03-07 14:22 +0100".to_string(),
            message: "build: completed layout".to_string(),
        };
        assert_eq!(entry.timestamp, "2024-03-07 14:22 +0100");
        assert_eq!(entry.message, "build: completed layout");
    }
}
