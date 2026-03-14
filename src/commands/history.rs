//! `fotobuch history` command - Show project change history

use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use git2::Repository;
use std::path::Path;

/// Single history entry
#[derive(Debug)]
pub struct HistoryEntry {
    pub timestamp: DateTime<FixedOffset>,
    pub message: String,
}

/// Show project change history via libgit2.
///
/// * `count` – max entries to return; 0 means all
pub fn history(project_root: &Path, count: usize) -> Result<Vec<HistoryEntry>> {
    let repo = match Repository::open(project_root) {
        Ok(r) => r,
        Err(_) => return Ok(Vec::new()),
    };

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head().unwrap_or(());
    revwalk.set_sorting(git2::Sort::TIME)?;

    let limit = if count == 0 { usize::MAX } else { count };

    let entries = revwalk
        .take(limit)
        .filter_map(|oid| {
            let commit = repo.find_commit(oid.ok()?).ok()?;
            let git_time = commit.time();
            let offset = FixedOffset::east_opt(git_time.offset_minutes() * 60)?;
            let timestamp = DateTime::from_timestamp(git_time.seconds(), 0)?.with_timezone(&offset);
            let message = commit.summary().unwrap_or("").to_string();
            Some(HistoryEntry { timestamp, message })
        })
        .collect();

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_returns_vec_for_nonexistent_path() {
        let result = history(Path::new("/nonexistent/path/xyz"), 5).unwrap();
        assert!(result.is_empty());
    }
}
