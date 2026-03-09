//! Integration tests for `fotobuch history` command

use anyhow::Result;
use std::process::Command;
use tempfile::TempDir;

/// Initialize a git repo with initial commit
fn init_git_repo(temp_dir: &TempDir) -> Result<()> {
    let dir = temp_dir.path();

    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()?;

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir)
        .output()?;

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir)
        .output()?;

    // Create initial commit
    std::fs::write(dir.join("README.md"), "# Test Project\n")?;
    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(dir)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", "initial: setup project"])
        .current_dir(dir)
        .output()?;

    Ok(())
}

#[test]
fn test_history_with_commits() -> Result<()> {
    let temp_dir = TempDir::new()?;
    init_git_repo(&temp_dir)?;

    let dir = temp_dir.path();

    // Add another commit
    std::fs::write(dir.join("file.txt"), "content\n")?;
    Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(dir)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", "feat: add new file"])
        .current_dir(dir)
        .output()?;

    // Call history function
    let entries = photobook_solver::commands::history(dir)?;

    assert!(!entries.is_empty());
    assert!(entries.len() >= 2); // At least 2 commits

    // Latest commit should be "feat: add new file"
    assert_eq!(entries[0].message, "feat: add new file");

    // Earlier commit should be "initial: setup project"
    assert!(entries.iter().any(|e| e.message == "initial: setup project"));

    // All entries should have timestamps
    for entry in &entries {
        assert!(!entry.timestamp.is_empty());
        assert!(entry.timestamp.contains("20")); // Year should be present
    }

    Ok(())
}

#[test]
fn test_history_no_git_repo() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let dir = temp_dir.path();

    // Call history in non-git directory
    let entries = photobook_solver::commands::history(dir)?;

    // Should return empty list, not error
    assert!(entries.is_empty());

    Ok(())
}

#[test]
fn test_history_empty_repo() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let dir = temp_dir.path();

    // Initialize empty git repo
    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()?;

    // Call history without commits
    let entries = photobook_solver::commands::history(dir)?;

    // Should return empty list (no commits)
    assert!(entries.is_empty());

    Ok(())
}

#[test]
fn test_history_timestamp_format() -> Result<()> {
    let temp_dir = TempDir::new()?;
    init_git_repo(&temp_dir)?;

    let entries = photobook_solver::commands::history(temp_dir.path())?;

    assert!(!entries.is_empty());

    // Timestamp should be ISO 8601 format: "YYYY-MM-DD HH:MM:SS +XXXX"
    let ts = &entries[0].timestamp;

    // Should contain date and time
    assert!(ts.contains("-")); // Date separator
    assert!(ts.contains(":")); // Time separator
    assert!(ts.contains("+")); // Timezone offset

    Ok(())
}
