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
    std::fs::write(dir.join("file.txt"), "content\n")?;
    Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(dir)
        .output()?;
    Command::new("git")
        .args(["commit", "-m", "feat: add new file"])
        .current_dir(dir)
        .output()?;

    let entries = fotobuch::commands::history(dir, 0)?;

    assert!(!entries.is_empty());
    assert!(entries.len() >= 2);
    assert_eq!(entries[0].message, "feat: add new file");
    assert!(
        entries
            .iter()
            .any(|e| e.message == "initial: setup project")
    );

    Ok(())
}

#[test]
fn test_history_no_git_repo() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let entries = fotobuch::commands::history(temp_dir.path(), 5)?;
    assert!(entries.is_empty());
    Ok(())
}

#[test]
fn test_history_empty_repo() -> Result<()> {
    let temp_dir = TempDir::new()?;
    Command::new("git")
        .args(["init"])
        .current_dir(temp_dir.path())
        .output()?;

    let entries = fotobuch::commands::history(temp_dir.path(), 5)?;
    assert!(entries.is_empty());
    Ok(())
}

#[test]
fn test_history_count_limit() -> Result<()> {
    let temp_dir = TempDir::new()?;
    init_git_repo(&temp_dir)?;

    let dir = temp_dir.path();
    for i in 1..=7 {
        std::fs::write(dir.join(format!("file{i}.txt")), "x")?;
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir)
            .output()?;
        Command::new("git")
            .args(["commit", "-m", &format!("feat: commit {i}")])
            .current_dir(dir)
            .output()?;
    }

    let all = fotobuch::commands::history(dir, 0)?;
    let limited = fotobuch::commands::history(dir, 5)?;

    assert!(all.len() >= 8); // 7 + initial
    assert_eq!(limited.len(), 5);
    assert_eq!(limited[0].message, all[0].message);

    Ok(())
}

#[test]
fn test_history_timestamp_has_valid_datetime() -> Result<()> {
    let temp_dir = TempDir::new()?;
    init_git_repo(&temp_dir)?;

    let entries = fotobuch::commands::history(temp_dir.path(), 5)?;

    assert!(!entries.is_empty());
    // timestamp is a DateTime<FixedOffset>; year should be >= 2024
    assert!(
        entries[0]
            .timestamp
            .format("%Y")
            .to_string()
            .parse::<i32>()
            .unwrap()
            >= 2024
    );

    Ok(())
}
