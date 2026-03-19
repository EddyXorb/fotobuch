//! `fotobuch undo` / `fotobuch redo` — git-based undo/redo.

use anyhow::{Context, Result, bail};
use git2::{Repository, ResetType, Status};
use std::path::Path;

use crate::{git, undo_stack};

#[derive(Debug)]
pub struct UndoResult {
    /// Whether uncommitted changes were auto-committed as "wip: before undo".
    pub wip_committed: bool,
    /// Summary of the commit that was undone / redone past.
    pub undone_message: String,
    /// Summary of the new HEAD after the operation.
    pub current_message: String,
}

/// Undo `steps` commits.
///
/// If the working tree is dirty the changes are committed automatically as
/// `wip: before undo` so they can be recovered via `redo`.
pub fn undo(project_root: &Path, steps: usize) -> Result<UndoResult> {
    if steps == 0 {
        bail!("Steps must be at least 1.");
    }

    let repo = git::open_repo(project_root)?;
    repo.head().context("No commits yet — nothing to undo.")?;

    let wip_committed = if is_dirty(&repo)? {
        commit_wip(&repo)?;
        true
    } else {
        false
    };

    let head_commit = repo.head()?.peel_to_commit()?;
    let undone_message = head_commit.summary().unwrap_or("").to_string();
    let head_sha = head_commit.id().to_string();

    let target = walk_back(&repo, steps)?;

    undo_stack::push(project_root, &head_sha)?;

    repo.reset(target.as_object(), ResetType::Hard, None)
        .context("Failed to reset working tree")?;

    let current_message = target.summary().unwrap_or("").to_string();

    Ok(UndoResult { wip_committed, undone_message, current_message })
}

/// Redo `steps` commits that were previously undone.
pub fn redo(project_root: &Path, steps: usize) -> Result<UndoResult> {
    if steps == 0 {
        bail!("Steps must be at least 1.");
    }

    let repo = git::open_repo(project_root)?;

    if is_dirty(&repo)? {
        bail!("Working tree has uncommitted changes. Commit or stash before redoing.");
    }

    let available = undo_stack::depth(project_root)?;
    if available == 0 {
        bail!("Nothing to redo.");
    }
    if steps > available {
        bail!("Only {available} redo step(s) available, cannot redo {steps}.");
    }

    let undone_message = repo.head()?.peel_to_commit()?.summary().unwrap_or("").to_string();

    // popped[0] = first popped = was top = last undo's origin (1 step ahead)
    // popped[steps-1] = `steps` steps ahead
    let popped = undo_stack::pop_n(project_root, steps)?;
    let target_sha = &popped[steps - 1];

    let oid = git2::Oid::from_str(target_sha).context("Invalid SHA in redo stack")?;
    let commit = repo.find_commit(oid).context("Redo commit not found — was the repo rewritten?")?;

    repo.reset(commit.as_object(), ResetType::Hard, None)
        .context("Failed to reset working tree")?;

    let current_message = commit.summary().unwrap_or("").to_string();

    Ok(UndoResult { wip_committed: false, undone_message, current_message })
}

fn is_dirty(repo: &Repository) -> Result<bool> {
    let statuses = repo.statuses(None).context("Failed to read repository status")?;
    Ok(statuses.iter().any(|s| {
        let st = s.status();
        // Ignore unmodified, ignored, and purely untracked files.
        // Untracked files (WT_NEW) are not affected by `git reset --hard` and in
        // production they live in .fotobuch/ (gitignored) anyway.
        !st.is_empty()
            && !st.intersects(Status::IGNORED)
            && st != Status::WT_NEW
    }))
}

fn walk_back(repo: &Repository, steps: usize) -> Result<git2::Commit<'_>> {
    let mut commit = repo
        .head()
        .context("No HEAD")?
        .peel_to_commit()
        .context("HEAD is not a commit")?;
    for i in 0..steps {
        commit = commit.parent(0).with_context(|| {
            format!("Only {i} commit(s) available, cannot undo {steps} step(s).")
        })?;
    }
    Ok(commit)
}

fn commit_wip(repo: &Repository) -> Result<()> {
    let mut index = repo.index().context("Failed to get index")?;
    // Update tracked files only — untracked files are not affected by reset --hard
    // and in production live in .fotobuch/ (gitignored).
    index
        .update_all(["*"].iter(), None)
        .context("Failed to stage tracked file changes")?;
    index.write().context("Failed to write index")?;

    let tree_id = index.write_tree().context("Failed to write tree")?;
    let tree = repo.find_tree(tree_id).context("Failed to find tree")?;
    let sig = git::build_signature(repo);
    let parent = repo.head()?.peel_to_commit()?;

    // Commit directly without going through stage_and_commit to avoid clearing
    // the redo stack (which would prevent redo after an undo of a WIP state).
    repo.commit(Some("HEAD"), &sig, &sig, "wip: before undo", &tree, &[&parent])
        .context("Failed to create WIP commit")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git;
    use std::fs;
    use tempfile::TempDir;

    fn setup_repo_with_commits(n: usize) -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = git::init_repo(dir.path()).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "test@test.com").unwrap();
        }
        fs::create_dir_all(dir.path().join(".fotobuch")).unwrap();
        for i in 1..=n {
            let file = dir.path().join("file.txt");
            fs::write(&file, format!("v{i}")).unwrap();
            git::stage_and_commit(&repo, &["file.txt"], &format!("commit {i}")).unwrap();
        }
        (dir, repo)
    }

    fn head_content(dir: &TempDir) -> String {
        fs::read_to_string(dir.path().join("file.txt")).unwrap()
    }

    #[test]
    fn undo_one_step() {
        let (dir, _repo) = setup_repo_with_commits(3);
        let result = undo(dir.path(), 1).unwrap();

        assert_eq!(result.undone_message, "commit 3");
        assert_eq!(result.current_message, "commit 2");
        assert_eq!(head_content(&dir), "v2");
    }

    #[test]
    fn undo_two_steps() {
        let (dir, _repo) = setup_repo_with_commits(3);
        undo(dir.path(), 2).unwrap();
        assert_eq!(head_content(&dir), "v1");
    }

    #[test]
    fn undo_then_redo() {
        let (dir, _repo) = setup_repo_with_commits(3);

        undo(dir.path(), 1).unwrap();
        assert_eq!(head_content(&dir), "v2");

        let result = redo(dir.path(), 1).unwrap();
        assert_eq!(result.current_message, "commit 3");
        assert_eq!(head_content(&dir), "v3");
    }

    #[test]
    fn redo_clears_after_new_commit() {
        let (dir, repo) = setup_repo_with_commits(3);

        undo(dir.path(), 1).unwrap();

        // Make a new commit — redo stack should be cleared
        fs::write(dir.path().join("file.txt"), "new").unwrap();
        git::stage_and_commit(&repo, &["file.txt"], "new commit").unwrap();

        let err = redo(dir.path(), 1).unwrap_err();
        assert!(err.to_string().contains("Nothing to redo"));
    }

    #[test]
    fn undo_too_many_steps_fails() {
        let (dir, _repo) = setup_repo_with_commits(2);
        let err = undo(dir.path(), 5).unwrap_err();
        assert!(err.to_string().contains("cannot undo"));
    }

    #[test]
    fn redo_more_than_available_fails() {
        let (dir, _repo) = setup_repo_with_commits(3);
        undo(dir.path(), 1).unwrap();
        let err = redo(dir.path(), 2).unwrap_err();
        assert!(err.to_string().contains("Only 1 redo step(s) available"));
    }

    #[test]
    fn undo_dirty_state_auto_commits_wip() {
        let (dir, _repo) = setup_repo_with_commits(2);

        // Modify file without committing
        fs::write(dir.path().join("file.txt"), "dirty").unwrap();

        let result = undo(dir.path(), 1).unwrap();
        assert!(result.wip_committed);

        // After undo 1: WIP was committed as HEAD, then HEAD~1 = v2
        assert_eq!(head_content(&dir), "v2");

        // Redo should restore the WIP commit (content = "dirty")
        let redo_result = redo(dir.path(), 1).unwrap();
        assert_eq!(redo_result.current_message, "wip: before undo");
        assert_eq!(head_content(&dir), "dirty");
    }

    #[test]
    fn redo_with_dirty_state_fails() {
        let (dir, _repo) = setup_repo_with_commits(3);
        undo(dir.path(), 1).unwrap();

        // Dirty state
        fs::write(dir.path().join("file.txt"), "dirty").unwrap();

        let err = redo(dir.path(), 1).unwrap_err();
        assert!(err.to_string().contains("uncommitted changes"));
    }

    #[test]
    fn multiple_undos_then_redo_all() {
        let (dir, _repo) = setup_repo_with_commits(4);

        undo(dir.path(), 1).unwrap(); // → v3
        undo(dir.path(), 1).unwrap(); // → v2
        undo(dir.path(), 1).unwrap(); // → v1
        assert_eq!(head_content(&dir), "v1");

        redo(dir.path(), 3).unwrap(); // → v4
        assert_eq!(head_content(&dir), "v4");
    }
}
