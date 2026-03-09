//! Integration tests for `fotobuch project list` command

use anyhow::Result;
use std::process::Command;
use tempfile::TempDir;

/// Initialize a git repo with fotobuch branches
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
        .args(["checkout", "-b", "fotobuch/vacation"])
        .current_dir(dir)
        .output()?;

    std::fs::write(dir.join("vacation.yaml"), "name: vacation\n")?;
    Command::new("git")
        .args(["add", "vacation.yaml"])
        .current_dir(dir)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", "add vacation project"])
        .current_dir(dir)
        .output()?;

    // Create second project branch
    Command::new("git")
        .args(["checkout", "-b", "fotobuch/wedding"])
        .current_dir(dir)
        .output()?;

    std::fs::write(dir.join("wedding.yaml"), "name: wedding\n")?;
    Command::new("git")
        .args(["add", "wedding.yaml"])
        .current_dir(dir)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", "add wedding project"])
        .current_dir(dir)
        .output()?;

    // Create third project branch
    Command::new("git")
        .args(["checkout", "-b", "fotobuch/birthday"])
        .current_dir(dir)
        .output()?;

    std::fs::write(dir.join("birthday.yaml"), "name: birthday\n")?;
    Command::new("git")
        .args(["add", "birthday.yaml"])
        .current_dir(dir)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", "add birthday project"])
        .current_dir(dir)
        .output()?;

    Ok(())
}

#[test]
fn test_project_list_finds_all_fotobuch_branches() -> Result<()> {
    let temp_dir = TempDir::new()?;
    init_git_with_projects(&temp_dir)?;

    let projects = photobook_solver::commands::project::project_list(temp_dir.path())?;

    assert_eq!(projects.len(), 3);

    // Check names exist
    let names: Vec<_> = projects.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"birthday"));
    assert!(names.contains(&"vacation"));
    assert!(names.contains(&"wedding"));

    Ok(())
}

#[test]
fn test_project_list_marks_current_project() -> Result<()> {
    let temp_dir = TempDir::new()?;
    init_git_with_projects(&temp_dir)?;

    let dir = temp_dir.path();

    // Switch to vacation branch
    Command::new("git")
        .args(["checkout", "fotobuch/vacation"])
        .current_dir(dir)
        .output()?;

    let projects = photobook_solver::commands::project::project_list(dir)?;

    // Find vacation project
    let vacation = projects.iter().find(|p| p.name == "vacation");
    assert!(vacation.is_some());
    assert!(vacation.unwrap().is_current);

    // Other projects should not be current
    let wedding = projects.iter().find(|p| p.name == "wedding");
    assert!(wedding.is_some());
    assert!(!wedding.unwrap().is_current);

    Ok(())
}

#[test]
fn test_project_list_ignores_non_fotobuch_branches() -> Result<()> {
    let temp_dir = TempDir::new()?;
    init_git_with_projects(&temp_dir)?;

    let dir = temp_dir.path();

    // Create a non-fotobuch branch
    Command::new("git")
        .args(["checkout", "-b", "feature/something"])
        .current_dir(dir)
        .output()?;

    let projects = photobook_solver::commands::project::project_list(dir)?;

    // Should still have only the 3 fotobuch projects
    assert_eq!(projects.len(), 3);

    // feature branch should not be in the list
    assert!(projects.iter().all(|p| !p.name.contains("feature")));

    Ok(())
}

#[test]
fn test_project_list_empty_repo() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let dir = temp_dir.path();

    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()?;

    let projects = photobook_solver::commands::project::project_list(dir)?;

    assert!(projects.is_empty());

    Ok(())
}

#[test]
fn test_project_list_not_a_git_repo() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let dir = temp_dir.path();

    // Don't initialize git
    let result = photobook_solver::commands::project::project_list(dir);

    // Should return error (not a git repo)
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_project_list_sorted_by_name() -> Result<()> {
    let temp_dir = TempDir::new()?;
    init_git_with_projects(&temp_dir)?;

    let projects = photobook_solver::commands::project::project_list(temp_dir.path())?;

    // Projects should be sorted alphabetically
    let names: Vec<_> = projects.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["birthday", "vacation", "wedding"]);

    Ok(())
}
