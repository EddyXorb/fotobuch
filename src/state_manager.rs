//! Central state management for fotobuch projects.
//!
//! [`StateManager`] is the single interface between CLI commands and the
//! persisted project state (YAML file). It handles:
//!
//! - Loading the YAML identified by the current git branch name
//! - Detecting and auto-committing manual user edits on `open()`
//! - Diff-detection between the state at `open()` and any programmatic changes
//! - Saving + committing on `finish()`
//! - Warning in `Drop` when programmatic changes were never committed

use anyhow::{Context, Result, bail};
use serde_yaml::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::dto_models::{LayoutPage, PhotoGroup, ProjectState};
use crate::git;

// ── StateDiff ────────────────────────────────────────────────────────────────

/// Summary of differences between two [`ProjectState`] snapshots.
#[derive(Debug, Default, PartialEq)]
struct StateDiff {
    config_changes: usize,
    photos_added: usize,
    photos_removed: usize,
    pages_added: usize,
    pages_removed: usize,
    pages_modified: usize,
}

impl StateDiff {
    /// Compute the diff between `old` and `new`.
    fn compute(old: &ProjectState, new: &ProjectState) -> Self {
        let config_changes = count_config_changes(old, new);

        let (photos_added, photos_removed) = diff_photos(&old.photos, &new.photos);
        let (pages_added, pages_removed, pages_modified) = diff_pages(&old.layout, &new.layout);

        Self {
            config_changes,
            photos_added,
            photos_removed,
            pages_added,
            pages_removed,
            pages_modified,
        }
    }

    fn is_empty(&self) -> bool {
        self.config_changes == 0
            && self.photos_added == 0
            && self.photos_removed == 0
            && self.pages_added == 0
            && self.pages_removed == 0
            && self.pages_modified == 0
    }

    /// Human-readable one-line summary, e.g. `"changed 2 configs, added 15 photos"`.
    fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.config_changes > 0 {
            parts.push(format!("changed {} config(s)", self.config_changes));
        }
        if self.photos_added > 0 {
            parts.push(format!("added {} photo(s)", self.photos_added));
        }
        if self.photos_removed > 0 {
            parts.push(format!("removed {} photo(s)", self.photos_removed));
        }
        if self.pages_added > 0 {
            parts.push(format!("added {} page(s)", self.pages_added));
        }
        if self.pages_removed > 0 {
            parts.push(format!("removed {} page(s)", self.pages_removed));
        }
        if self.pages_modified > 0 {
            parts.push(format!("modified {} page(s)", self.pages_modified));
        }
        if parts.is_empty() {
            "no changes".to_owned()
        } else {
            parts.join(", ")
        }
    }
}

/// Count differing leaf values in the config section by serialising both states
/// to `serde_yaml::Value` and recursively comparing leaves.
fn count_config_changes(old: &ProjectState, new: &ProjectState) -> usize {
    let old_val = serde_yaml::to_value(&old.config).unwrap_or(Value::Null);
    let new_val = serde_yaml::to_value(&new.config).unwrap_or(Value::Null);
    count_value_diffs(&old_val, &new_val)
}

fn count_value_diffs(a: &Value, b: &Value) -> usize {
    match (a, b) {
        (Value::Mapping(ma), Value::Mapping(mb)) => {
            let keys: HashSet<_> = ma.keys().chain(mb.keys()).collect();
            keys.into_iter()
                .map(|k| count_value_diffs(ma.get(k).unwrap_or(&Value::Null), mb.get(k).unwrap_or(&Value::Null)))
                .sum()
        }
        _ => usize::from(a != b),
    }
}

/// Returns (added, removed) photo counts by comparing file IDs across states.
fn diff_photos(old: &[PhotoGroup], new: &[PhotoGroup]) -> (usize, usize) {
    let old_ids: HashSet<&str> = old.iter().flat_map(|g| g.files.iter().map(|f| f.id.as_str())).collect();
    let new_ids: HashSet<&str> = new.iter().flat_map(|g| g.files.iter().map(|f| f.id.as_str())).collect();
    let added = new_ids.difference(&old_ids).count();
    let removed = old_ids.difference(&new_ids).count();
    (added, removed)
}

/// Returns (pages_added, pages_removed, pages_modified).
///
/// Modified = a page that exists in both old and new but has different slots.
fn diff_pages(old: &[LayoutPage], new: &[LayoutPage]) -> (usize, usize, usize) {
    let old_map: std::collections::HashMap<usize, &LayoutPage> =
        old.iter().map(|p| (p.page, p)).collect();
    let new_map: std::collections::HashMap<usize, &LayoutPage> =
        new.iter().map(|p| (p.page, p)).collect();

    let added = new_map.keys().filter(|k| !old_map.contains_key(k)).count();
    let removed = old_map.keys().filter(|k| !new_map.contains_key(k)).count();
    let modified = old_map
        .iter()
        .filter(|(k, old_page)| {
            new_map.get(k).is_some_and(|new_page| {
                old_page.slots != new_page.slots || old_page.photos != new_page.photos
            })
        })
        .count();

    (added, removed, modified)
}

// ── StateManager ─────────────────────────────────────────────────────────────

/// Central project state manager.
///
/// `state` is intentionally `pub` so commands can take disjoint borrows on
/// `mgr.state.photos` and `mgr.state.layout` simultaneously without borrowing
/// the whole manager.
pub struct StateManager {
    project_root: PathBuf,
    project_name: String,
    repo: git2::Repository,

    /// Current (potentially mutated) project state.
    pub state: ProjectState,
    /// Snapshot of state at `open()` time — used to detect programmatic changes.
    last_state: ProjectState,
    /// Raw YAML value of the config section as loaded from disk.
    raw_config: Value,
    /// Set to `true` by `finish()` so `Drop` stays silent.
    committed: bool,
}

impl StateManager {
    /// Open a project: read branch → load YAML → auto-commit manual edits.
    ///
    /// Fails if:
    /// - The directory is not a git repository
    /// - The current branch does not follow the `fotobuch/<name>` convention
    /// - The YAML file for the project cannot be read or parsed
    pub fn open(project_root: &Path) -> Result<Self> {
        let repo = git::open_repo(project_root)?;
        let branch = git::current_branch(&repo)?;
        let project_name = branch
            .strip_prefix("fotobuch/")
            .with_context(|| {
                format!("Current branch '{branch}' does not start with 'fotobuch/' — run 'fotobuch project switch <name>' first")
            })?
            .to_owned();

        let yaml_path = project_root.join(format!("{project_name}.yaml"));
        let state = ProjectState::load(&yaml_path)
            .with_context(|| format!("Failed to load {}", yaml_path.display()))?;

        // Store raw config value for the config command
        let raw_config = load_raw_config(&yaml_path)?;

        // Load the committed version and auto-commit any manual edits
        let mut mgr = Self {
            project_root: project_root.to_owned(),
            project_name,
            repo,
            last_state: state.clone(),
            state,
            raw_config,
            committed: false,
        };

        mgr.auto_commit_manual_edits()?;
        // After potential auto-commit, reset baseline
        mgr.last_state = mgr.state.clone();
        Ok(mgr)
    }

    /// Project name derived from the current branch (`fotobuch/<name>` → `<name>`).
    pub fn project_name(&self) -> &str {
        &self.project_name
    }

    /// Path to `{project_root}/.fotobuch/cache/{project_name}/`.
    pub fn cache_dir(&self) -> PathBuf {
        self.project_root
            .join(".fotobuch")
            .join("cache")
            .join(&self.project_name)
    }

    /// Path to `{cache_dir}/preview/`.
    pub fn preview_cache_dir(&self) -> PathBuf {
        self.cache_dir().join("preview")
    }

    /// Path to `{cache_dir}/final/`.
    pub fn final_cache_dir(&self) -> PathBuf {
        self.cache_dir().join("final")
    }

    /// Absolute path to `{project_root}/{project_name}.yaml`.
    pub fn yaml_path(&self) -> PathBuf {
        self.project_root.join(format!("{}.yaml", self.project_name))
    }

    /// Raw `serde_yaml::Value` of the `config` section as it was on disk at `open()`.
    ///
    /// Useful for detecting which fields were explicitly set vs. defaulted.
    pub fn raw_config(&self) -> &Value {
        &self.raw_config
    }

    /// `true` when `state` differs from the snapshot taken at `open()`.
    pub fn has_changes(&self) -> bool {
        !StateDiff::compute(&self.last_state, &self.state).is_empty()
    }

    /// Save YAML and commit if `state` changed since `open()`.  Consumes the manager.
    ///
    /// The commit message is `"{message} — {diff_summary}"`.
    /// When there are no changes this is a no-op.
    pub fn finish(mut self, message: &str) -> Result<()> {
        let diff = StateDiff::compute(&self.last_state, &self.state);
        if diff.is_empty() {
            self.committed = true;
            return Ok(());
        }

        let yaml_name = format!("{}.yaml", self.project_name);
        self.state
            .save(&self.project_root.join(&yaml_name))
            .context("Failed to save YAML")?;

        let commit_msg = format!("{} — {}", message, diff.summary());
        git::stage_and_commit(&self.repo, &[&yaml_name], &commit_msg)?;

        self.committed = true;
        Ok(())
    }

    // ── private helpers ───────────────────────────────────────────────────────

    /// If the on-disk YAML differs from the last committed version, auto-commit
    /// the manual edits with `"chore: manual edits — {summary}"`.
    fn auto_commit_manual_edits(&mut self) -> Result<()> {
        let committed_state = self.load_committed_state()?;
        let Some(committed) = committed_state else {
            // No previous commit for this file — nothing to compare
            return Ok(());
        };

        let diff = StateDiff::compute(&committed, &self.state);
        if diff.is_empty() {
            return Ok(());
        }

        let yaml_name = format!("{}.yaml", self.project_name);
        let commit_msg = format!("chore: manual edits — {}", diff.summary());
        git::stage_and_commit(&self.repo, &[&yaml_name], &commit_msg)?;

        Ok(())
    }

    /// Load the project YAML from the latest commit (`HEAD:{name}.yaml`).
    ///
    /// Returns `None` when the file doesn't exist in HEAD yet (initial project state).
    fn load_committed_state(&self) -> Result<Option<ProjectState>> {
        let blob_spec = format!("HEAD:{}.yaml", self.project_name);
        let obj = match self.repo.revparse_single(&blob_spec) {
            Ok(o) => o,
            Err(e) if e.code() == git2::ErrorCode::NotFound => return Ok(None),
            Err(e) => {
                return Err(e).with_context(|| format!("Failed to resolve '{blob_spec}'"));
            }
        };

        let blob = obj
            .into_blob()
            .map_err(|_| anyhow::anyhow!("'{blob_spec}' is not a blob"))?;

        let content =
            std::str::from_utf8(blob.content()).context("Committed YAML is not valid UTF-8")?;

        let state: ProjectState =
            serde_yaml::from_str(content).context("Failed to parse committed YAML")?;

        Ok(Some(state))
    }
}

impl Drop for StateManager {
    fn drop(&mut self) {
        if !self.committed {
            let diff = StateDiff::compute(&self.last_state, &self.state);
            if !diff.is_empty() {
                eprintln!(
                    "warning: StateManager dropped with uncommitted changes: {}",
                    diff.summary()
                );
            }
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn load_raw_config(yaml_path: &Path) -> Result<Value> {
    let content = std::fs::read_to_string(yaml_path)
        .with_context(|| format!("Failed to read {}", yaml_path.display()))?;
    let doc: Value = serde_yaml::from_str(&content).context("Failed to parse YAML")?;
    Ok(match doc {
        Value::Mapping(ref m) => m
            .get(Value::String("config".to_owned()))
            .cloned()
            .unwrap_or(Value::Null),
        _ => bail!("YAML root is not a mapping"),
    })
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::{BookConfig, BookLayoutSolverConfig, LayoutPage, PhotoFile, PhotoGroup, ProjectConfig, ProjectState, Slot};
    use tempfile::TempDir;

    fn make_state(title: &str) -> ProjectState {
        ProjectState {
            config: ProjectConfig {
                book: BookConfig {
                    title: title.to_owned(),
                    page_width_mm: 420.0,
                    page_height_mm: 297.0,
                    bleed_mm: 3.0,
                    margin_mm: 10.0,
                    gap_mm: 5.0,
                    bleed_threshold_mm: 3.0,
                },
                page_layout_solver: Default::default(),
                preview: Default::default(),
                book_layout_solver: BookLayoutSolverConfig::default(),
            },
            photos: vec![],
            layout: vec![],
        }
    }

    fn make_photo(id: &str) -> PhotoFile {
        PhotoFile {
            id: id.to_owned(),
            source: format!("/photos/{id}"),
            timestamp: "2024-01-01T00:00:00Z".parse().unwrap(),
            width_px: 4000,
            height_px: 3000,
            area_weight: 1.0,
            hash: None,
        }
    }

    // ── StateDiff tests ───────────────────────────────────────────────────────

    #[test]
    fn test_statediff_empty_when_identical() {
        let s = make_state("Test");
        let diff = StateDiff::compute(&s, &s);
        assert!(diff.is_empty());
        assert_eq!(diff.summary(), "no changes");
    }

    #[test]
    fn test_statediff_config_change() {
        let old = make_state("Urlaub");
        let mut new = old.clone();
        new.config.book.title = "Hochzeit".to_owned();
        let diff = StateDiff::compute(&old, &new);
        assert_eq!(diff.config_changes, 1);
        assert!(diff.summary().contains("config"));
    }

    #[test]
    fn test_statediff_photos_added() {
        let old = make_state("T");
        let mut new = old.clone();
        new.photos.push(PhotoGroup {
            group: "Strand".to_owned(),
            sort_key: "2024-07-15T00:00:00Z".to_owned(),
            files: vec![make_photo("Strand/a.jpg"), make_photo("Strand/b.jpg")],
        });
        let diff = StateDiff::compute(&old, &new);
        assert_eq!(diff.photos_added, 2);
        assert_eq!(diff.photos_removed, 0);
    }

    #[test]
    fn test_statediff_photos_removed() {
        let mut old = make_state("T");
        old.photos.push(PhotoGroup {
            group: "Strand".to_owned(),
            sort_key: "2024-07-15T00:00:00Z".to_owned(),
            files: vec![make_photo("Strand/a.jpg")],
        });
        let new = make_state("T");
        let diff = StateDiff::compute(&old, &new);
        assert_eq!(diff.photos_removed, 1);
        assert_eq!(diff.photos_added, 0);
    }

    #[test]
    fn test_statediff_pages_added() {
        let old = make_state("T");
        let mut new = old.clone();
        new.layout.push(LayoutPage { page: 1, photos: vec![], slots: vec![] });
        let diff = StateDiff::compute(&old, &new);
        assert_eq!(diff.pages_added, 1);
        assert_eq!(diff.pages_removed, 0);
        assert_eq!(diff.pages_modified, 0);
    }

    #[test]
    fn test_statediff_pages_modified() {
        let mut old = make_state("T");
        old.layout.push(LayoutPage {
            page: 1,
            photos: vec!["p1".to_owned()],
            slots: vec![Slot { x_mm: 0.0, y_mm: 0.0, width_mm: 100.0, height_mm: 80.0 }],
        });
        let mut new = old.clone();
        new.layout[0].slots[0].width_mm = 200.0;
        let diff = StateDiff::compute(&old, &new);
        assert_eq!(diff.pages_modified, 1);
        assert_eq!(diff.pages_added, 0);
    }

    #[test]
    fn test_statediff_summary_multiple_changes() {
        let mut old = make_state("T");
        old.photos.push(PhotoGroup {
            group: "G".to_owned(),
            sort_key: "2024-01-01T00:00:00Z".to_owned(),
            files: vec![make_photo("G/a.jpg")],
        });
        let mut new = make_state("T2"); // title change = 1 config diff
        new.photos.push(PhotoGroup {
            group: "G".to_owned(),
            sort_key: "2024-01-01T00:00:00Z".to_owned(),
            files: vec![make_photo("G/a.jpg"), make_photo("G/b.jpg")], // +1 photo
        });
        let diff = StateDiff::compute(&old, &new);
        assert_eq!(diff.config_changes, 1);
        assert_eq!(diff.photos_added, 1);
        let s = diff.summary();
        assert!(s.contains("config"));
        assert!(s.contains("photo"));
    }

    // ── StateManager integration test ─────────────────────────────────────────

    fn setup_project_repo(tmp: &TempDir) -> git2::Repository {
        let repo = git::init_repo(tmp.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        drop(config);

        // Write .gitignore + initial yaml
        std::fs::write(tmp.path().join(".gitignore"), ".fotobuch/\n*.pdf\nfinal.typ\n").unwrap();
        let state = make_state("Urlaub");
        state.save(&tmp.path().join("urlaub.yaml")).unwrap();

        // Initial commit on master, then create fotobuch/urlaub branch
        git::stage_and_commit(&repo, &[".gitignore", "urlaub.yaml"], "init").unwrap();
        git::create_branch(&repo, "fotobuch/urlaub").unwrap();

        repo
    }

    #[test]
    fn test_open_reads_project_name_from_branch() {
        let tmp = TempDir::new().unwrap();
        setup_project_repo(&tmp);
        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.project_name(), "urlaub");
    }

    #[test]
    fn test_open_fails_on_non_fotobuch_branch() {
        let tmp = TempDir::new().unwrap();
        let repo = git::init_repo(tmp.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "T").unwrap();
        config.set_str("user.email", "t@t.de").unwrap();
        drop(config);
        std::fs::write(tmp.path().join("x.txt"), "x").unwrap();
        git::stage_and_commit(&repo, &["x.txt"], "init").unwrap();
        // Still on master — not a fotobuch branch
        let result = StateManager::open(tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_yaml_path() {
        let tmp = TempDir::new().unwrap();
        setup_project_repo(&tmp);
        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.yaml_path(), tmp.path().join("urlaub.yaml"));
    }

    #[test]
    fn test_cache_dirs() {
        let tmp = TempDir::new().unwrap();
        setup_project_repo(&tmp);
        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.cache_dir(), tmp.path().join(".fotobuch/cache/urlaub"));
        assert_eq!(mgr.preview_cache_dir(), tmp.path().join(".fotobuch/cache/urlaub/preview"));
        assert_eq!(mgr.final_cache_dir(), tmp.path().join(".fotobuch/cache/urlaub/final"));
    }

    #[test]
    fn test_has_changes_after_mutation() {
        let tmp = TempDir::new().unwrap();
        setup_project_repo(&tmp);
        let mut mgr = StateManager::open(tmp.path()).unwrap();
        assert!(!mgr.has_changes());
        mgr.state.config.book.title = "Changed".to_owned();
        assert!(mgr.has_changes());
        mgr.committed = true; // Silence Drop warning
    }

    #[test]
    fn test_finish_commits_changes() {
        let tmp = TempDir::new().unwrap();
        setup_project_repo(&tmp);
        let mut mgr = StateManager::open(tmp.path()).unwrap();
        mgr.state.config.book.title = "Changed".to_owned();
        mgr.finish("test: change title").unwrap();

        // Verify commit was created
        let repo = git::open_repo(tmp.path()).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert!(head.message().unwrap_or("").contains("test: change title"));
    }

    #[test]
    fn test_finish_noop_when_no_changes() {
        let tmp = TempDir::new().unwrap();
        setup_project_repo(&tmp);
        let repo_before = git::open_repo(tmp.path()).unwrap();
        let commit_before = repo_before.head().unwrap().peel_to_commit().unwrap().id();
        drop(repo_before);

        let mgr = StateManager::open(tmp.path()).unwrap();
        mgr.finish("should not commit").unwrap();

        let repo_after = git::open_repo(tmp.path()).unwrap();
        let commit_after = repo_after.head().unwrap().peel_to_commit().unwrap().id();
        assert_eq!(commit_before, commit_after);
    }

    #[test]
    fn test_auto_commit_manual_edits() {
        let tmp = TempDir::new().unwrap();
        setup_project_repo(&tmp);

        // Simulate a manual edit: write a modified YAML to disk without committing
        {
            let mut state = make_state("Urlaub");
            state.config.book.title = "ManualEdit".to_owned();
            state.save(&tmp.path().join("urlaub.yaml")).unwrap();
        }

        // open() should detect the diff vs HEAD and auto-commit
        let repo_before = git::open_repo(tmp.path()).unwrap();
        let commit_count_before = count_commits(&repo_before);
        drop(repo_before);

        let mgr = StateManager::open(tmp.path()).unwrap();
        drop(mgr); // no programmatic changes

        let repo_after = git::open_repo(tmp.path()).unwrap();
        let commit_count_after = count_commits(&repo_after);
        // One new commit for the manual edit
        assert_eq!(commit_count_after, commit_count_before + 1);
        let msg = repo_after
            .head().unwrap()
            .peel_to_commit().unwrap()
            .message().unwrap_or("").to_owned();
        assert!(msg.starts_with("chore: manual edits"));
    }

    fn count_commits(repo: &git2::Repository) -> usize {
        let mut walk = repo.revwalk().unwrap();
        walk.push_head().unwrap();
        walk.count()
    }
}
