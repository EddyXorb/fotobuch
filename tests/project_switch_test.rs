//! Integration tests for `fotobuch project switch` command

use anyhow::Result;
use std::process::Command;
use tempfile::TempDir;

/// Initialize a git repo with multiple fotobuch branches
fn init_git_with_projects(temp_dir: &TempDir) -> Result<()> {
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
    std::fs::write(dir.join("README.md"), "# Test\n")?;
    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(dir)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir)
        .output()?;

    // Create first project branch
    Command::new("git")
        .args(["checkout", "-b", "fotobuch/project1"])
        .current_dir(dir)
        .output()?;

    std::fs::write(dir.join("project1.yaml"), "name: project1\n")?;
    Command::new("git")
        .args(["add", "project1.yaml"])
        .current_dir(dir)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", "add project1"])
        .current_dir(dir)
        .output()?;

    // Create second project branch from main, not from project1
    Command::new("git")
        .args(["checkout", "master"])
        .current_dir(dir)
        .output()?;

    Command::new("git")
        .args(["checkout", "-b", "fotobuch/project2"])
        .current_dir(dir)
        .output()?;

    std::fs::write(dir.join("project2.yaml"), "name: project2\n")?;
    Command::new("git")
        .args(["add", "project2.yaml"])
        .current_dir(dir)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", "add project2"])
        .current_dir(dir)
        .output()?;

    Ok(())
}

#[test]
fn test_project_switch_to_existing_project() -> Result<()> {
    let temp_dir = TempDir::new()?;
    init_git_with_projects(&temp_dir)?;

    let dir = temp_dir.path();

    // Switch from project2 to project1
    photobook_solver::commands::project::project_switch(dir, "project1")?;

    // Verify we're on project1 branch
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(dir)
        .output()?;

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(branch, "fotobuch/project1");

    // Verify project1.yaml exists (essential file on this branch)
    assert!(dir.join("project1.yaml").exists());

    Ok(())
}

#[test]
fn test_project_switch_to_non_existent() -> Result<()> {
    let temp_dir = TempDir::new()?;
    init_git_with_projects(&temp_dir)?;

    let dir = temp_dir.path();

    // Try to switch to non-existent project
    let result = photobook_solver::commands::project::project_switch(dir, "nonexistent");

    // Should be an error
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));

    Ok(())
}

#[test]
fn test_project_switch_with_uncommitted_changes() -> Result<()> {
    let temp_dir = TempDir::new()?;
    init_git_with_projects(&temp_dir)?;

    let dir = temp_dir.path();

    // Make a change to a file
    std::fs::write(
        dir.join("project2.yaml"),
        "name: project2\nmodified: true\n",
    )?;

    // Try to switch with uncommitted changes
    let result = photobook_solver::commands::project::project_switch(dir, "project1");

    // Should fail with uncommitted changes error
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("uncommitted") || err_msg.contains("changes"));

    Ok(())
}

#[test]
fn test_project_switch_to_current_project() -> Result<()> {
    let temp_dir = TempDir::new()?;
    init_git_with_projects(&temp_dir)?;

    let dir = temp_dir.path();

    // Get current branch
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(dir)
        .output()?;
    let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Extract project name from branch
    let current_project = if current_branch.starts_with("fotobuch/") {
        current_branch.strip_prefix("fotobuch/").unwrap()
    } else {
        return Ok(()); // Skip if not on a fotobuch branch
    };

    // Switch to current project (should be idempotent)
    let result = photobook_solver::commands::project::project_switch(dir, current_project);

    // Should succeed without error
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_project_switch_invalid_name() -> Result<()> {
    let temp_dir = TempDir::new()?;
    init_git_with_projects(&temp_dir)?;

    // Try with invalid project name (contains slash)
    let result =
        photobook_solver::commands::project::project_switch(temp_dir.path(), "invalid/name");

    // Should fail validation
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_project_switch_to_non_git_directory() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Don't initialize git
    let result = photobook_solver::commands::project::project_switch(temp_dir.path(), "project1");

    // Should fail
    assert!(result.is_err());

    Ok(())
}
