//! Redo stack for `fotobuch undo` / `fotobuch redo`.
//!
//! The stack is stored in `.fotobuch/redo-stack` as plain text (one SHA per line,
//! most-recently-pushed last).  The file lives in `.fotobuch/` which is already
//! gitignored, so git history stays clean.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

fn stack_path(project_root: &Path) -> PathBuf {
    project_root.join(".fotobuch").join("redo-stack")
}

fn read_stack(project_root: &Path) -> Result<Vec<String>> {
    let path = stack_path(project_root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read redo stack at {}", path.display()))?;
    Ok(content
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect())
}

fn write_stack(project_root: &Path, entries: &[String]) -> Result<()> {
    let path = stack_path(project_root);
    if entries.is_empty() {
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to remove redo stack at {}", path.display()))?;
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }
    let content = entries.join("\n") + "\n";
    fs::write(&path, content)
        .with_context(|| format!("Failed to write redo stack at {}", path.display()))
}

/// Push a commit SHA onto the top of the redo stack.
pub fn push(project_root: &Path, sha: &str) -> Result<()> {
    let mut stack = read_stack(project_root)?;
    stack.push(sha.to_owned());
    write_stack(project_root, &stack)
}

/// Pop up to `n` SHAs from the redo stack.
///
/// Returns the popped SHAs in pop order (index 0 = first popped = was the top).
/// Returns fewer than `n` items if the stack is exhausted.
pub fn pop_n(project_root: &Path, n: usize) -> Result<Vec<String>> {
    let mut stack = read_stack(project_root)?;
    let count = n.min(stack.len());
    let split_at = stack.len() - count;
    // Drain from the end (top of stack), reverse so index 0 = first popped (was top).
    let popped: Vec<String> = stack.drain(split_at..).rev().collect();
    write_stack(project_root, &stack)?;
    Ok(popped)
}

/// Clear the redo stack (called whenever a new normal commit is made).
pub fn clear(project_root: &Path) -> Result<()> {
    write_stack(project_root, &[])
}

/// Number of available redo steps.
pub fn depth(project_root: &Path) -> Result<usize> {
    Ok(read_stack(project_root)?.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn empty_stack_depth_is_zero() {
        let dir = tmp();
        assert_eq!(depth(dir.path()).unwrap(), 0);
    }

    #[test]
    fn push_and_depth() {
        let dir = tmp();
        push(dir.path(), "aaa").unwrap();
        push(dir.path(), "bbb").unwrap();
        assert_eq!(depth(dir.path()).unwrap(), 2);
    }

    #[test]
    fn pop_n_returns_top_first() {
        let dir = tmp();
        push(dir.path(), "first").unwrap();
        push(dir.path(), "second").unwrap();
        push(dir.path(), "third").unwrap();

        let popped = pop_n(dir.path(), 2).unwrap();
        assert_eq!(popped, vec!["third", "second"]);
        assert_eq!(depth(dir.path()).unwrap(), 1);
    }

    #[test]
    fn pop_n_more_than_available_returns_all() {
        let dir = tmp();
        push(dir.path(), "only").unwrap();

        let popped = pop_n(dir.path(), 5).unwrap();
        assert_eq!(popped, vec!["only"]);
        assert_eq!(depth(dir.path()).unwrap(), 0);
    }

    #[test]
    fn clear_empties_stack() {
        let dir = tmp();
        push(dir.path(), "x").unwrap();
        clear(dir.path()).unwrap();
        assert_eq!(depth(dir.path()).unwrap(), 0);
        assert!(!stack_path(dir.path()).exists());
    }

    #[test]
    fn clear_on_empty_stack_is_noop() {
        let dir = tmp();
        clear(dir.path()).unwrap(); // no panic
        assert_eq!(depth(dir.path()).unwrap(), 0);
    }
}
