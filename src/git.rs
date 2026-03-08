//! Git integration using the git2 crate.

use anyhow::{Context, Result, bail};
use git2::{IndexAddOption, Repository, Signature};
use std::path::Path;

/// Open an existing git repository at the given directory.
pub fn open_repo(dir: &Path) -> Result<Repository> {
    Repository::open(dir).with_context(|| format!("Failed to open git repo at {}", dir.display()))
}

/// Initialize a new git repository at the given directory.
pub fn init_repo(dir: &Path) -> Result<Repository> {
    Repository::init(dir).with_context(|| format!("Failed to init git repo at {}", dir.display()))
}

/// Return the current branch name (short name, e.g. `fotobuch/urlaub`).
pub fn current_branch(repo: &Repository) -> Result<String> {
    let head = repo.head().context("Failed to read HEAD")?;
    if head.is_branch() {
        head.shorthand()
            .map(str::to_owned)
            .context("Branch name is not valid UTF-8")
    } else {
        bail!("HEAD is not pointing to a branch (detached HEAD?)")
    }
}

/// Create and checkout a new branch pointing at HEAD.
pub fn create_branch(repo: &Repository, name: &str) -> Result<()> {
    let head_commit = repo
        .head()
        .context("Failed to read HEAD")?
        .peel_to_commit()
        .context("HEAD is not a commit")?;
    repo.branch(name, &head_commit, false)
        .with_context(|| format!("Failed to create branch '{name}'"))?;
    switch_branch(repo, name)
}

/// Switch to an existing branch.
pub fn switch_branch(repo: &Repository, name: &str) -> Result<()> {
    let refname = format!("refs/heads/{name}");
    let obj = repo
        .revparse_single(&refname)
        .with_context(|| format!("Branch '{name}' not found"))?;
    repo.checkout_tree(&obj, None)
        .with_context(|| format!("Failed to checkout tree for '{name}'"))?;
    repo.set_head(&refname)
        .with_context(|| format!("Failed to set HEAD to '{name}'"))?;
    Ok(())
}

/// List all local branch names that start with `prefix`.
///
/// Returns the full branch names (e.g. `fotobuch/urlaub`).
pub fn list_branches_with_prefix(repo: &Repository, prefix: &str) -> Result<Vec<String>> {
    let branches = repo
        .branches(Some(git2::BranchType::Local))
        .context("Failed to list branches")?;

    let mut names = Vec::new();
    for branch in branches {
        let (branch, _) = branch.context("Failed to read branch")?;
        if let Some(name) = branch.name().context("Branch name is not UTF-8")?
            && name.starts_with(prefix)
        {
            names.push(name.to_owned());
        }
    }
    Ok(names)
}

/// Stage the given relative paths and create a commit.
///
/// Uses a dummy author/committer from the repository config when available,
/// falling back to a placeholder.
pub fn stage_and_commit(repo: &Repository, paths: &[&str], message: &str) -> Result<()> {
    let mut index = repo.index().context("Failed to get index")?;
    index
        .add_all(paths.iter().copied(), IndexAddOption::DEFAULT, None)
        .context("Failed to stage files")?;
    index.write().context("Failed to write index")?;

    let tree_id = index.write_tree().context("Failed to write tree")?;
    let tree = repo.find_tree(tree_id).context("Failed to find tree")?;

    let sig = build_signature(repo);
    let parent_commits: Vec<git2::Commit<'_>> = match repo.head() {
        Ok(head) => vec![head.peel_to_commit().context("HEAD is not a commit")?],
        Err(_) => vec![], // initial commit has no parent
    };
    let parent_refs: Vec<&git2::Commit<'_>> = parent_commits.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
        .context("Failed to create commit")?;
    Ok(())
}

/// Check if the directory contains a `.git` folder (is a git repository root).
pub fn is_git_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

fn build_signature(repo: &Repository) -> Signature<'static> {
    let config = repo.config().ok();
    let name = config
        .as_ref()
        .and_then(|c| c.get_string("user.name").ok())
        .unwrap_or_else(|| "fotobuch".to_owned());
    let email = config
        .as_ref()
        .and_then(|c| c.get_string("user.email").ok())
        .unwrap_or_else(|| "fotobuch@localhost".to_owned());
    Signature::now(&name, &email).unwrap_or_else(|_| {
        Signature::now("fotobuch", "fotobuch@localhost").expect("fallback signature must work")
    })
}

/// Legacy helper: stage `{name}.yaml` and commit.  Used by the `add` command.
pub fn commit(project_dir: &Path, message: &str) -> Result<()> {
    let repo = open_repo(project_dir)?;
    // Determine project name from current branch (fotobuch/<name>)
    let branch = current_branch(&repo)?;
    let yaml_name = if let Some(name) = branch.strip_prefix("fotobuch/") {
        format!("{name}.yaml")
    } else {
        "fotobuch.yaml".to_owned()
    };
    stage_and_commit(&repo, &[&yaml_name], message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_repo() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = init_repo(dir.path()).unwrap();
        // git2 repos start with unborn HEAD — set a proper config so signatures work
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        (dir, repo)
    }

    #[test]
    fn test_is_git_repo() {
        let tmp = TempDir::new().unwrap();
        assert!(!is_git_repo(tmp.path()));
        fs::create_dir(tmp.path().join(".git")).unwrap();
        assert!(is_git_repo(tmp.path()));
    }

    #[test]
    fn test_init_and_current_branch_unborn() {
        let (dir, _repo) = make_repo();
        // Unborn HEAD has no branch yet; open_repo + current_branch should err
        let repo2 = open_repo(dir.path()).unwrap();
        // HEAD is unborn after git init (no commits) → current_branch returns Err
        let result = current_branch(&repo2);
        assert!(result.is_err());
    }

    #[test]
    fn test_stage_and_commit_initial() {
        let (dir, repo) = make_repo();
        let file = dir.path().join("hello.txt");
        fs::write(&file, "hello").unwrap();

        stage_and_commit(&repo, &["hello.txt"], "init: hello").unwrap();

        let branch = current_branch(&repo).unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn test_create_and_switch_branch() {
        let (dir, repo) = make_repo();
        let file = dir.path().join("readme.txt");
        fs::write(&file, "content").unwrap();
        stage_and_commit(&repo, &["readme.txt"], "init").unwrap();

        create_branch(&repo, "fotobuch/urlaub").unwrap();
        let branch = current_branch(&repo).unwrap();
        assert_eq!(branch, "fotobuch/urlaub");
    }

    #[test]
    fn test_list_branches_with_prefix() {
        let (dir, repo) = make_repo();
        let file = dir.path().join("a.txt");
        fs::write(&file, "a").unwrap();
        stage_and_commit(&repo, &["a.txt"], "init").unwrap();

        create_branch(&repo, "fotobuch/urlaub").unwrap();

        // switch back to main to create another fotobuch branch
        switch_branch(&repo, "main").unwrap();
        create_branch(&repo, "fotobuch/hochzeit").unwrap();

        let mut branches = list_branches_with_prefix(&repo, "fotobuch/").unwrap();
        branches.sort();
        assert_eq!(branches, vec!["fotobuch/hochzeit", "fotobuch/urlaub"]);
    }

    #[test]
    fn test_list_branches_empty_prefix() {
        let (dir, repo) = make_repo();
        let file = dir.path().join("a.txt");
        fs::write(&file, "a").unwrap();
        stage_and_commit(&repo, &["a.txt"], "init").unwrap();

        let branches = list_branches_with_prefix(&repo, "other/").unwrap();
        assert!(branches.is_empty());
    }
}
