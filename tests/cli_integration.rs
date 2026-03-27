//! Integration tests for the `fotobuch` CLI.
//!
//! Each test runs the binary as a subprocess via `assert_cmd`.
//! Output is captured from stdout (tracing writes there with timestamps).

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("fotobuch").unwrap()
}

/// Create a minimal project in `dir` and return the path to its root.
fn create_project(dir: &TempDir, name: &str) -> std::path::PathBuf {
    cmd()
        .current_dir(dir.path())
        .args([
            "project", "new", name, "--width", "210", "--height", "148", "--quiet",
        ])
        .assert()
        .success();
    dir.path().join(name)
}

// ── Smoke tests ──────────────────────────────────────────────────────────────

#[test]
fn help_flag() {
    cmd().arg("--help").assert().success();
}

#[test]
fn version_flag() {
    cmd().arg("--version").assert().success();
}

#[test]
fn unknown_subcommand_fails() {
    cmd().arg("foobar").assert().failure();
}

// ── project new ──────────────────────────────────────────────────────────────

#[test]
fn project_new_creates_directory() {
    let dir = TempDir::new().unwrap();
    let project = create_project(&dir, "my-book");
    assert!(
        project.is_dir(),
        "project directory must exist after creation"
    );
}

#[test]
fn project_new_missing_width_fails() {
    let dir = TempDir::new().unwrap();
    cmd()
        .current_dir(dir.path())
        .args(["project", "new", "my-book", "--height", "148"])
        .assert()
        .failure();
}

#[test]
fn project_new_missing_name_fails() {
    let dir = TempDir::new().unwrap();
    cmd()
        .current_dir(dir.path())
        .args(["project", "new", "--width", "210", "--height", "148"])
        .assert()
        .failure();
}

// ── project list ─────────────────────────────────────────────────────────────

#[test]
fn project_list_outside_repo_fails() {
    let dir = TempDir::new().unwrap();
    cmd()
        .current_dir(dir.path())
        .args(["project", "list"])
        .assert()
        .failure();
}

#[test]
fn project_list_shows_created_project() {
    let dir = TempDir::new().unwrap();
    let project = create_project(&dir, "my-book");
    cmd()
        .current_dir(&project)
        .args(["project", "list"])
        .assert()
        .success()
        .stdout(contains("my-book"));
}

// ── status ───────────────────────────────────────────────────────────────────

#[test]
fn status_empty_project_succeeds() {
    let dir = TempDir::new().unwrap();
    let project = create_project(&dir, "my-book");
    cmd()
        .current_dir(&project)
        .args(["status"])
        .assert()
        .success();
}

#[test]
fn status_outside_project_fails() {
    let dir = TempDir::new().unwrap();
    cmd()
        .current_dir(dir.path())
        .args(["status"])
        .assert()
        .failure();
}

// ── config ───────────────────────────────────────────────────────────────────

#[test]
fn config_in_project_succeeds() {
    let dir = TempDir::new().unwrap();
    let project = create_project(&dir, "my-book");
    cmd()
        .current_dir(&project)
        .args(["config"])
        .assert()
        .success();
}

// ── history ──────────────────────────────────────────────────────────────────

#[test]
fn history_in_project_succeeds() {
    let dir = TempDir::new().unwrap();
    let project = create_project(&dir, "my-book");
    cmd()
        .current_dir(&project)
        .args(["history"])
        .assert()
        .success();
}

// ── add ──────────────────────────────────────────────────────────────────────

#[test]
fn add_dry_run_no_paths_succeeds() {
    let dir = TempDir::new().unwrap();
    let project = create_project(&dir, "my-book");
    cmd()
        .current_dir(&project)
        .args(["add", "--dry"])
        .assert()
        .success();
}

#[test]
fn add_invalid_filter_regex_fails() {
    let dir = TempDir::new().unwrap();
    let project = create_project(&dir, "my-book");
    cmd()
        .current_dir(&project)
        .args(["add", "--filter", "[invalid"])
        .assert()
        .failure();
}

// ── page commands: address parsing errors ────────────────────────────────────

#[test]
fn page_move_invalid_address_fails() {
    let dir = TempDir::new().unwrap();
    let project = create_project(&dir, "my-book");
    cmd()
        .current_dir(&project)
        .args(["page", "move", "not-a-valid-address"])
        .assert()
        .failure();
}

#[test]
fn page_split_invalid_address_fails() {
    let dir = TempDir::new().unwrap();
    let project = create_project(&dir, "my-book");
    cmd()
        .current_dir(&project)
        .args(["page", "split", "bad"])
        .assert()
        .failure();
}

#[test]
fn page_weight_zero_fails() {
    let dir = TempDir::new().unwrap();
    let project = create_project(&dir, "my-book");
    cmd()
        .current_dir(&project)
        .args(["page", "weight", "1:1", "0"])
        .assert()
        .failure();
}

// ── rebuild: conflicting flags fail ──────────────────────────────────────────

#[test]
fn rebuild_page_and_all_conflict_fails() {
    let dir = TempDir::new().unwrap();
    let project = create_project(&dir, "my-book");
    cmd()
        .current_dir(&project)
        .args(["rebuild", "--page", "1", "--all"])
        .assert()
        .failure();
}

#[test]
fn rebuild_range_start_without_end_fails() {
    let dir = TempDir::new().unwrap();
    let project = create_project(&dir, "my-book");
    cmd()
        .current_dir(&project)
        .args(["rebuild", "--range-start", "1"])
        .assert()
        .failure();
}
