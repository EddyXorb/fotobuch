//! Vault resolution and initialization.
//!
//! A **vault** is a single directory containing a git repo with any number of
//! `fotobuch/*` branches.  It corresponds 1:1 to `repo_root` in the CLI.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::app_settings::AppSettings;
use crate::git;

/// Default vault path: `~/Pictures/Fotobuch` (XDG Pictures dir with fallback).
pub fn default_vault_path() -> PathBuf {
    dirs::picture_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Fotobuch")
}

/// Resolve which vault to open, applying the documented priority chain:
///
/// 1. CLI argument `--vault`
/// 2. `FOTOBUCH_VAULT` environment variable (`env_var` arg)
/// 3. `cwd` if it already contains `fotobuch/*` branches
/// 4. `settings.last_vault` if it still exists on disk
/// 5. Default vault `~/Pictures/Fotobuch`
///
/// All three inputs are explicit arguments so the function is pure and testable.
pub fn resolve_vault(
    cli_arg: Option<&Path>,
    env_var: Option<&str>,
    cwd: &Path,
    settings: &AppSettings,
) -> PathBuf {
    if let Some(p) = cli_arg {
        return p.to_path_buf();
    }
    if let Some(s) = env_var {
        return PathBuf::from(s);
    }
    if has_fotobuch_branches(cwd) {
        return cwd.to_path_buf();
    }
    if let Some(v) = &settings.last_vault
        && v.exists()
    {
        return v.clone();
    }
    default_vault_path()
}

/// Ensure the vault directory and its git repo exist.
///
/// If the directory is missing it is created.  If it has no git repo one is
/// initialized with an initial commit (so that `project_new` can create
/// branches immediately).  Idempotent on an already-initialized vault.
pub fn ensure_vault(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)
            .map_err(|e| anyhow::anyhow!("Failed to create vault dir {}: {e}", path.display()))?;
    }
    if !git::is_git_repo(path) {
        let repo = git::init_repo(path)?;
        // Write .gitignore so the first project commit is clean
        let gitignore = path.join(".gitignore");
        std::fs::write(
            &gitignore,
            ".fotobuch/\n*.pdf\nfinal.typ\nlog*\n*.yaml\n*.typ\n",
        )
        .map_err(|e| anyhow::anyhow!("Failed to write .gitignore: {e}"))?;
        git::stage_and_commit(&repo, &[".gitignore"], "chore: init vault")?;
    }
    Ok(())
}

fn has_fotobuch_branches(path: &Path) -> bool {
    if !git::is_git_repo(path) {
        return false;
    }
    git::open_repo(path)
        .and_then(|r| git::list_branches_with_prefix(&r, "fotobuch/"))
        .map(|b| !b.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
        use tempfile::TempDir;

    fn empty_settings() -> AppSettings {
        AppSettings::default()
    }

    // ── resolve_vault priority tests ──────────────────────────────────────────

    #[test]
    fn resolve_vault_priorities_cli_over_env_over_cwd_over_settings_over_default() {
        let tmp = TempDir::new().unwrap();
        let cli_path = tmp.path().join("cli");
        let env_path = tmp.path().join("env");
        let cwd_path = tmp.path().join("cwd");
        let settings_path = tmp.path().join("settings");
        std::fs::create_dir_all(&cli_path).unwrap();
        std::fs::create_dir_all(&env_path).unwrap();
        std::fs::create_dir_all(&cwd_path).unwrap();
        std::fs::create_dir_all(&settings_path).unwrap();

        let mut settings = AppSettings::default();
        settings.last_vault = Some(settings_path.clone());

        // 1. CLI wins over everything
        let result = resolve_vault(
            Some(&cli_path),
            Some(env_path.to_str().unwrap()),
            &cwd_path,
            &settings,
        );
        assert_eq!(result, cli_path);

        // 2. Env wins over cwd/settings (no CLI)
        let result = resolve_vault(None, Some(env_path.to_str().unwrap()), &cwd_path, &settings);
        assert_eq!(result, env_path);

        // 3. settings.last_vault wins over default (no CLI, no env, cwd has no branches)
        let result = resolve_vault(None, None, &cwd_path, &settings);
        assert_eq!(result, settings_path);

        // 4. Default when nothing matches
        let mut empty_s = AppSettings::default();
        // Point last_vault at non-existent path so it's skipped
        empty_s.last_vault = Some(tmp.path().join("nonexistent"));
        let result = resolve_vault(None, None, &cwd_path, &empty_s);
        assert_eq!(result, default_vault_path());
    }

    #[test]
    fn resolve_vault_cwd_with_fotobuch_branches_wins_over_settings() {
        let tmp = TempDir::new().unwrap();
        let settings_vault = tmp.path().join("settings_vault");
        std::fs::create_dir_all(&settings_vault).unwrap();

        // Create a vault in tmp and add a fotobuch branch via git directly
        ensure_vault(tmp.path()).unwrap();
        let repo = git::open_repo(tmp.path()).unwrap();
        git::create_branch(&repo, "fotobuch/testproject").unwrap();

        let mut settings = AppSettings::default();
        settings.last_vault = Some(settings_vault.clone());

        // cwd = tmp.path() which has fotobuch branches → wins over settings
        let result = resolve_vault(None, None, tmp.path(), &settings);
        assert_eq!(result, tmp.path());
    }

    // ── ensure_vault tests ────────────────────────────────────────────────────

    #[test]
    fn ensure_vault_creates_directory_and_repo() {
        let tmp = TempDir::new().unwrap();
        let vault = tmp.path().join("my-vault");
        assert!(!vault.exists());
        ensure_vault(&vault).unwrap();
        assert!(vault.exists());
        assert!(git::is_git_repo(&vault));
    }

    #[test]
    fn ensure_vault_is_idempotent_on_existing_repo() {
        let tmp = TempDir::new().unwrap();
        ensure_vault(tmp.path()).unwrap();
        // Second call must not fail or create extra commits
        let repo = git::open_repo(tmp.path()).unwrap();
        let commit_count_before = {
            let mut walk = repo.revwalk().unwrap();
            walk.push_head().unwrap();
            walk.count()
        };
        ensure_vault(tmp.path()).unwrap();
        let repo = git::open_repo(tmp.path()).unwrap();
        let commit_count_after = {
            let mut walk = repo.revwalk().unwrap();
            walk.push_head().unwrap();
            walk.count()
        };
        assert_eq!(commit_count_before, commit_count_after);
    }

    #[test]
    fn ensure_vault_writes_gitignore() {
        let tmp = TempDir::new().unwrap();
        ensure_vault(tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert!(content.contains(".fotobuch/"));
        assert!(content.contains("*.pdf"));
    }
}
