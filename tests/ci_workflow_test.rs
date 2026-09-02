//! Tests für den CI-Workflow (`.github/workflows/ci.yml`).
//!
//! Hintergrund: `cargo clippy` und `cargo llvm-cov` bauen nur die
//! Default-Features. Optionale Features (`gui`, `cli-docs`, `slow-tests`)
//! blieben dadurch komplett ungeprüft -- ein Dependency-Update konnte den
//! GUI-Build brechen, ohne die CI rot zu färben (siehe PR #68: `egui` auf
//! 0.35, `eframe` weiter auf 0.34).

use std::path::PathBuf;

fn ci_workflow() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/ci.yml");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Ohne diesen Schritt bleibt jedes optionale Feature ungebaut.
#[test]
fn ci_compiles_all_features() {
    let workflow = ci_workflow();
    assert!(
        workflow.contains("cargo check --all-features --all-targets"),
        "ci.yml baut nicht alle Features -- Brüche im gui-Feature bleiben unbemerkt"
    );
}

/// Alle optionalen Features müssen von `--all-features` erfasst werden, also
/// in `Cargo.toml` unter `[features]` deklariert sein. Ein Feature, das nur
/// über eine optionale Dependency existiert, würde durchrutschen.
#[test]
fn every_optional_feature_is_declared() {
    let manifest =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("cannot read Cargo.toml");

    let features = manifest
        .split("[features]")
        .nth(1)
        .expect("Cargo.toml hat keinen [features]-Abschnitt");

    for expected in ["gui", "cli-docs", "slow-tests"] {
        assert!(
            features.contains(expected),
            "Feature `{expected}` fehlt in [features] und würde von --all-features nicht erfasst"
        );
    }
}
