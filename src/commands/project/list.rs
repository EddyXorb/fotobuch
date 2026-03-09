//! `fotobuch project list` command - List all projects

use anyhow::Result;
use std::path::Path;

/// List all photobook projects from git branches
///
/// Searches for branches matching pattern `fotobuch/*` and returns info about each one.
/// Marks which project is currently checked out.
///
/// # Arguments
/// * `project_root` - Path to any directory in the git repository
///
/// # Returns
/// * Vector of ProjectInfo structs with name, branch, and is_current flag
pub fn project_list(project_root: &Path) -> Result<Vec<super::ProjectInfo>> {
    use git2::Repository;

    let repo = Repository::open(project_root)?;

    // Get current HEAD branch
    let current_branch = repo
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(|s| s.to_string()));

    let mut projects = Vec::new();

    // Iterate over all branches
    for branch_result in repo.branches(Some(git2::BranchType::Local))? {
        let (branch, _) = branch_result?;
        let branch_name = branch.name()?.unwrap_or("");

        // Filter for fotobuch/* prefix
        if !branch_name.starts_with("fotobuch/") {
            continue;
        }

        // Extract project name (everything after "fotobuch/")
        let project_name = branch_name
            .strip_prefix("fotobuch/")
            .unwrap_or("")
            .to_string();

        if project_name.is_empty() {
            continue;
        }

        let is_current = current_branch.as_ref().map(|cb| cb == branch_name).unwrap_or(false);

        projects.push(super::ProjectInfo {
            name: project_name,
            branch: branch_name.to_string(),
            is_current,
        });
    }

    // Sort by name for consistent output
    projects.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(projects)
}
