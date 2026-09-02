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

// --- Crate-Familien ------------------------------------------------------
//
// `typst`/`typst-pdf`/`typst-render`/`typst-kit` und `egui`/`eframe`/... sind
// jeweils Familien, die nur im Gleichschritt funktionieren: wird ein einzelnes
// Mitglied angehoben, liegen zwei Versionen derselben Bibliothek im
// Abhängigkeitsbaum, und gleichnamige Typen aus verschiedenen Versionen sind
// nicht zuweisungskompatibel. Einzel-PRs für Familienmitglieder können darum
// grundsätzlich nicht grün werden.

/// Direkte Abhängigkeiten aus `Cargo.toml` -- sowohl `name = "..."` innerhalb
/// von `[dependencies]`/`[dev-dependencies]` als auch `[dependencies.name]`.
fn direct_dependencies() -> Vec<String> {
    let manifest =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("cannot read Cargo.toml");

    let mut names = Vec::new();
    let mut in_dependency_table = false;

    for line in manifest.lines().map(str::trim) {
        if let Some(section) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let section = section.trim_start_matches("[").trim_end_matches("]");
            in_dependency_table = section == "dependencies" || section == "dev-dependencies";
            if let Some(name) = section
                .strip_prefix("dependencies.")
                .or_else(|| section.strip_prefix("dev-dependencies."))
            {
                names.push(name.to_string());
            }
            continue;
        }
        if !in_dependency_table || line.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = line.split_once(" = ")
            && !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            names.push(name.to_string());
        }
    }
    names
}

/// Dependabot-Glob: nur ein abschließendes `*` ist zu unterstützen.
fn pattern_matches(pattern: &str, name: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => pattern == name,
    }
}

/// Name der ersten Gruppe, die `name` erfasst -- Dependabot ordnet jede
/// Abhängigkeit genau der ersten passenden Gruppe zu.
fn group_of(entry: &Value, name: &str) -> Option<String> {
    let groups = entry["groups"].as_mapping()?;
    for (group_name, group) in groups {
        if group["applies-to"].as_str() == Some("security-updates") {
            continue;
        }
        let matches = match group["patterns"].as_sequence() {
            Some(patterns) => patterns
                .iter()
                .filter_map(Value::as_str)
                .any(|p| pattern_matches(p, name)),
            // Ohne `patterns` erfasst eine Gruppe alles.
            None => true,
        };
        if matches {
            return Some(group_name.as_str()?.to_string());
        }
    }
    None
}

fn assert_family_is_grouped(family: &str, members: &[&str]) {
    let config = dependabot_config();
    let cargo = entry_for(&config, "cargo");

    let groups: Vec<Option<String>> = members.iter().map(|m| group_of(cargo, m)).collect();
    let first = groups[0].clone();

    assert!(
        first.is_some(),
        "{family}: `{}` ist keiner Gruppe zugeordnet",
        members[0]
    );
    for (member, group) in members.iter().zip(&groups) {
        assert_eq!(
            group, &first,
            "{family}: `{member}` landet in Gruppe {group:?}, `{}` aber in {first:?} -- \
             getrennte PRs für Familienmitglieder können nicht kompilieren",
            members[0]
        );
    }

    // Eine Gruppe mit `update-types: [minor, patch]` lässt Breaking Changes
    // (0.14 -> 0.15) durchfallen; genau die werden dann zu Einzel-PRs.
    let group_name = first.expect("Familie ist keiner Gruppe zugeordnet");
    let group = &cargo["groups"][group_name.as_str()];
    assert!(
        group["update-types"].is_null(),
        "{family}: Gruppe `{group_name}` schränkt update-types ein -- \
         Breaking Changes der Familie kämen weiterhin als Einzel-PRs"
    );
}

/// Jede direkte Abhängigkeit der Familie muss abgedeckt sein, auch eine
/// später hinzugefügte.
fn family_members(prefixes: &[&str]) -> Vec<String> {
    let mut members: Vec<String> = direct_dependencies()
        .into_iter()
        .filter(|d| {
            prefixes.iter().any(|p| {
                d == p || d.starts_with(&format!("{p}-")) || d.starts_with(&format!("{p}_"))
            })
        })
        .collect();
    members.sort();
    members.dedup();
    members
}

#[test]
fn typst_crates_share_one_group() {
    let members = family_members(&["typst"]);
    assert!(
        members.len() > 1,
        "keine typst-Familie in Cargo.toml gefunden -- Test veraltet?"
    );
    let refs: Vec<&str> = members.iter().map(String::as_str).collect();
    assert_family_is_grouped("typst", &refs);
}

#[test]
fn egui_crates_share_one_group() {
    let mut members = family_members(&["egui"]);
    if direct_dependencies().iter().any(|d| d == "eframe") {
        members.push("eframe".to_string());
    }
    assert!(
        members.len() > 1,
        "keine egui-Familie in Cargo.toml gefunden -- Test veraltet?"
    );
    let refs: Vec<&str> = members.iter().map(String::as_str).collect();
    assert_family_is_grouped("egui", &refs);
}

/// Familiengruppen müssen vor der Sammelgruppe stehen, sonst schluckt diese
/// die Mitglieder zuerst.
#[test]
fn family_groups_precede_the_catch_all_group() {
    let config = dependabot_config();
    let cargo = entry_for(&config, "cargo");
    let groups = cargo["groups"]
        .as_mapping()
        .expect("cargo hat keine groups");

    let order: Vec<&str> = groups.keys().filter_map(Value::as_str).collect();
    let catch_all = order
        .iter()
        .position(|g| *g == "cargo-minor-patch")
        .expect("Gruppe cargo-minor-patch fehlt");

    for family in ["typst", "egui"] {
        let idx = order
            .iter()
            .position(|g| *g == family)
            .unwrap_or_else(|| panic!("Gruppe {family} fehlt"));
        assert!(
            idx < catch_all,
            "Gruppe {family} steht hinter cargo-minor-patch und greift deshalb nie"
        );
    }
}
