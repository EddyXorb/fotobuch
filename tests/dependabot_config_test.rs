//! Tests for the Dependabot configuration (`.github/dependabot.yml`).
//!
//! The `Security Audit` workflow (`cargo audit`) fails as soon as an advisory
//! affects any crate in `Cargo.lock`. Dependabot is the mechanism that keeps
//! those crates fresh, so its configuration is checked in CI like any other
//! part of the build.

use serde_yaml::Value;
use std::path::PathBuf;

fn dependabot_config() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".github/dependabot.yml");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_yaml::from_str(&raw).expect("dependabot.yml is not valid YAML")
}

fn updates(config: &Value) -> &Vec<Value> {
    config["updates"]
        .as_sequence()
        .expect("dependabot.yml has no `updates` list")
}

fn entry_for<'a>(config: &'a Value, ecosystem: &str) -> &'a Value {
    updates(config)
        .iter()
        .find(|entry| entry["package-ecosystem"].as_str() == Some(ecosystem))
        .unwrap_or_else(|| panic!("no update entry for ecosystem `{ecosystem}`"))
}

#[test]
fn config_uses_schema_version_2() {
    assert_eq!(dependabot_config()["version"].as_u64(), Some(2));
}

/// Jedes Ökosystem, das im Repository tatsächlich vorkommt, muss abgedeckt
/// sein -- sonst laufen für dessen Abhängigkeiten keine Wochen-Updates.
/// `tests/tools` ist ein uv-Projekt (pyproject.toml + uv.lock).
#[test]
fn all_ecosystems_in_the_repository_are_covered() {
    let config = dependabot_config();
    for (ecosystem, directory) in [
        ("cargo", "/"),
        ("github-actions", "/"),
        ("uv", "/tests/tools"),
    ] {
        let entry = entry_for(&config, ecosystem);
        assert_eq!(
            entry["directory"].as_str(),
            Some(directory),
            "ecosystem `{ecosystem}` watches the wrong directory"
        );
    }
}

/// Guard gegen ein neues Manifest ohne passenden Dependabot-Eintrag.
#[test]
fn python_tooling_manifest_still_lives_where_dependabot_looks() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/tools/pyproject.toml");
    assert!(
        manifest.is_file(),
        "{} fehlt -- dependabot.yml zeigt ins Leere",
        manifest.display()
    );
}

#[test]
fn every_ecosystem_is_updated_weekly() {
    let config = dependabot_config();
    for entry in updates(&config) {
        assert_eq!(
            entry["schedule"]["interval"].as_str(),
            Some("weekly"),
            "ecosystem {:?} is not scheduled weekly",
            entry["package-ecosystem"]
        );
    }
}

/// Most `cargo audit` findings (quick-xml, crossbeam-epoch, webbrowser, ...)
/// sit in transitive crates. Without `dependency-type: all` Dependabot only
/// looks at the direct dependencies in `Cargo.toml` and the audit stays red.
#[test]
fn cargo_updates_include_transitive_dependencies() {
    let config = dependabot_config();
    let allow = entry_for(&config, "cargo")["allow"]
        .as_sequence()
        .expect("cargo entry has no `allow` list");

    assert!(
        allow
            .iter()
            .any(|rule| rule["dependency-type"].as_str() == Some("all")),
        "cargo entry must allow `dependency-type: all`"
    );
}

/// Grouped PRs keep the weekly noise down; ungrouped updates would open a
/// separate pull request per crate.
#[test]
fn updates_are_grouped() {
    let config = dependabot_config();
    for entry in updates(&config) {
        let groups = entry["groups"]
            .as_mapping()
            .unwrap_or_else(|| panic!("ecosystem {:?} has no groups", entry["package-ecosystem"]));
        assert!(!groups.is_empty());
    }
}

/// Dependabot soll ausschließlich Pull Requests öffnen -- gemergt wird von
/// Hand. GitHub merged nie von selbst; ein Auto-Merge entsteht nur durch einen
/// zusätzlichen Workflow (`gh pr merge --auto`, `enable-pull-request-automerge`,
/// `merge_pull_request`). Dieser Test schlägt an, falls so ein Workflow
/// nachträglich hinzukommt.
#[test]
fn no_workflow_merges_dependabot_pull_requests() {
    let workflows = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".github/workflows");
    let markers = [
        "pr merge",
        "pull-request-automerge",
        "automerge",
        "auto-merge",
        "merge_pull_request",
    ];

    for file in std::fs::read_dir(&workflows).expect("no .github/workflows directory") {
        let path = file.expect("unreadable directory entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("yml") {
            continue;
        }
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
            .to_lowercase();

        for marker in markers {
            assert!(
                !content.contains(marker),
                "{} enthält `{marker}` -- Dependabot-PRs sollen von Hand gemergt werden",
                path.display()
            );
        }
    }
}

/// Dependabot erkennt den Conventional-Commit-Stil des Repositories selbst und
/// setzt `chore(deps): ...` bereits von sich aus -- belegt durch PR #56/#57/#58,
/// die entstanden, bevor `.github/dependabot.yml` existierte. Ein zusaetzliches
/// `include: scope` haengt einen zweiten Scope an, woraus
/// `chore(deps)(deps): ...` wird und damit kein gueltiger Conventional Commit
/// mehr.
#[test]
fn commit_message_scope_is_left_to_dependabot() {
    let config = dependabot_config();

    for entry in updates(&config) {
        let commit_message = &entry["commit-message"];
        assert!(
            commit_message.is_null(),
            "ecosystem {:?} setzt einen commit-message-Block; Dependabot setzt \
             Typ und Scope selbst, jede eigene Angabe verdoppelt den Scope",
            entry["package-ecosystem"]
        );
    }
}
