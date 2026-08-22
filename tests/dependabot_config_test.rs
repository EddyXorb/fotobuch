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

#[test]
fn cargo_and_github_actions_are_covered() {
    let config = dependabot_config();
    for ecosystem in ["cargo", "github-actions"] {
        let entry = entry_for(&config, ecosystem);
        assert_eq!(entry["directory"].as_str(), Some("/"));
    }
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
