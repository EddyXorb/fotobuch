//! Git integration for tracking project changes.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Commit changes to the fotobuch project repository
pub fn commit(project_dir: &Path, message: &str) -> Result<()> {
    // Check if git is available
    let git_check = Command::new("git")
        .arg("--version")
        .output()
        .context("Failed to check if git is installed")?;

    if !git_check.status.success() {
        anyhow::bail!("Git is not available");
    }

    // Add fotobuch.yaml
    let add_status = Command::new("git")
        .current_dir(project_dir)
        .arg("add")
        .arg("fotobuch.yaml")
        .status()
        .context("Failed to execute 'git add'")?;

    if !add_status.success() {
        anyhow::bail!("Failed to stage fotobuch.yaml");
    }

    // Commit with the provided message
    let commit_status = Command::new("git")
        .current_dir(project_dir)
        .arg("commit")
        .arg("-m")
        .arg(message)
        .status()
        .context("Failed to execute 'git commit'")?;

    if !commit_status.success() {
        anyhow::bail!("Git commit failed");
    }

    Ok(())
}

/// Check if the directory is a git repository
pub fn is_git_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_is_git_repo() {
        let temp_dir = TempDir::new().unwrap();
        assert!(!is_git_repo(temp_dir.path()));

        // Create .git directory
        fs::create_dir(temp_dir.path().join(".git")).unwrap();
        assert!(is_git_repo(temp_dir.path()));
    }

    // Note: Testing actual git commands requires git to be installed
    // and would create real commits, so we skip integration tests here.
}
