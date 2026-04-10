//! `fotobuch project switch` command - Switch to another project

use anyhow::{Result, bail};
use std::path::Path;

use crate::commands::CommandOutput;
use crate::state_manager::load_project_state;

/// Switch to another photobook project
///
/// Checks out the specified project's branch (`fotobuch/<name>`)
/// and updates the working tree.
///
/// # Arguments
/// * `project_root` - Path to any directory in the git repository
/// * `name` - Project name to switch to
///
/// # Returns
/// * Ok(()) if switch successful
/// * Error if project not found, uncommitted changes, or git error
pub fn project_switch(project_root: &Path, name: &str) -> Result<CommandOutput<()>> {
    use git2::Repository;

    // Validate project name
    super::new::validate_project_name(name)?;

    let repo = Repository::open(project_root)?;
    let branch_name = format!("fotobuch/{}", name);

    // Check if branch exists
    let _branch = repo
        .find_branch(&branch_name, git2::BranchType::Local)
        .map_err(|_| {
            anyhow::anyhow!(
                "Project '{}' not found. Use 'fotobuch project list' to see available projects.",
                name
            )
        })?;

    // Check for uncommitted changes (dirty working tree)
    let statuses = repo.statuses(None)?;
    let mut has_changes = false;
    for path_status in statuses.iter() {
        let status_flags = path_status.status();
        // Ignore WT_NEW and other untracked files, focus on actual changes
        if status_flags.contains(git2::Status::WT_MODIFIED)
            || status_flags.contains(git2::Status::WT_DELETED)
            || status_flags.contains(git2::Status::INDEX_MODIFIED)
            || status_flags.contains(git2::Status::INDEX_DELETED)
        {
            has_changes = true;
            break;
        }
    }

    if has_changes {
        bail!("Working tree has uncommitted changes. Commit or stash before switching.");
    }

    // Get current HEAD
    let is_already_on_branch = if let Ok(head) = repo.head() {
        head.shorthand() == Some(&branch_name[..])
    } else {
        false
    };

    if is_already_on_branch {
        // Already on this branch - this is fine, just return
        let changed_state = Some(load_project_state(project_root)?);
        return Ok(CommandOutput {
            result: (),
            changed_state,
        });
    }

    // Switch to the branch: update working tree first, then point HEAD
    let object = repo.revparse_single(&branch_name)?;
    repo.checkout_tree(&object, None)?;
    repo.set_head(&format!("refs/heads/{}", branch_name))?;

    let changed_state = Some(load_project_state(project_root)?);
    Ok(CommandOutput {
        result: (),
        changed_state,
    })
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_project_switch_validates_name() {
        // Names with slashes should be invalid
        let result = super::super::new::validate_project_name("invalid/name");
        assert!(result.is_err());
    }
}
