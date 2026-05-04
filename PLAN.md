# Plan: Slow Tests

## Findings

673 Tests, 1 skipped. Gesamtlaufzeit ~2min (nextest parallel). Drei Kategorien langsamer Tests:

### Kategorie A: build_test / place_test (15–30s pro Test)

| Test | Zeit |
|------|------|
| `build_test::test_build_from_scratch_with_max_groups_per_page_one` | 29s |
| `build_test::test_pages_filter_limits_scope` | 21s |
| `build_test::test_release_requires_pages_flag_not_allowed` | 19s |
| `build_test::test_incremental_build_without_changes_does_nothing` | 18s |
| `build_test::test_first_build_creates_layout_and_pdf` | 17s |
| `build_test::test_release_requires_clean_state` | 17s |
| `build_test::test_max_groups_per_page_limits_to_one_group` | 16s |
| `place_test::test_place_no_unplaced_photos_returns_zero` | 20s |
| `place_test::test_place_invalid_filter_pattern` | 17s |
| `place_test::test_place_ids_filter_restricts_to_selected_photos` | 17s |
| `place_test::test_place_filter_by_pattern` | 17s |
| `place_test::test_place_chronologically_without_unplaced` | 16s |
| `place_test::test_place_into_new_page_at_invalid_position` | 16s |
| `place_test::test_place_into_new_page_at` | 16s |
| `place_test::test_place_into_specific_page` | 15s |
| `build_test::test_build_handles_empty_photo_list` | 5s |

**Ursache:** Jeder Test ruft `build()` auf, was die komplette Pipeline ausführt:
1. Solver (MIP + GA page layout) → ~0.5–1s
2. Typst-Kompilierung zu PDF → ~2–3s
3. Preview-Cache-Rendering (rasterize jede Seite) → ~1–2s
4. Git-Operationen (commit) → ~0.1s

Bei `place_test` kommt der Overhead vom `create_test_project_with_layout()` helper, der intern einen vollen `build()` macht bevor der eigentliche Place-Test startet.

**Schlüsselproblem:** Alle Tests starten bei Null – kein shared Setup, kein Fixture-Reuse.

### Kategorie B: output::typst* Tests (2–3s pro Test)

| Test | Zeit |
|------|------|
| `output::typst::test_render_pages_page_selection` | 2.8s |
| `output::typst::test_compile_preview` | 2.7s |
| `output::typst::test_compile_final` | 2.7s |
| `output::typst_world::reload_invalidates_changed_image` | 2.6s |
| `output::typst_world::reload_invalidates_changed_main` | 2.6s |
| `output::typst::test_render_pages_straight_alpha_conversion` | 2.6s |
| `output::typst::test_render_pages_correct_dimensions` | 2.5s |
| `output::typst::test_add_pdf_boxes_sets_trim_and_bleed_box` | 2.4s |
| `output::typst::test_compile_simple_template` | 2.4s |
| `output::typst_world::compile_document_succeeds_on_minimal_template` | 2.3s |
| `output::typst_world::reload_keeps_unchanged_slots` | 2.4s |
| `output::typst_world::new_initializes_empty_slots` | 2.1s |

**Ursache:** Typst-Initialisierung (Fonts laden, World aufbauen) kostet ~2s pro `TypstWorld::new()`. Jeder Test erstellt sein eigenes `TypstWorld`.

### Kategorie C: Solver-Tests (0.6–12s)

| Test | Zeit |
|------|------|
| `page_assignment_solver::test_no_split_needed` | 12.6s |
| `page_assignment_solver::test_integration_large_instance_with_split` | >30s |
| `book_layout_solver::test_solve_book_layout_single_page` | 0.65s |
| `book_layout_solver::integration::test_solve_multiple_groups` | 0.21s |

**Ursache:** MIP-Solver (MILP-Optimierung) mit realistischen Problemgrößen.

### Kategorie D: Sonstiges (0.1–1.4s)

- `add_test::test_xmp_filter_matches_modified_description` (1.4s) — exiftool-Subprozess
- `cache::preview::test_ensure_previews_creates_missing` (3.2s) — Typst + Rasterisierung
- `cache::preview::test_ensure_previews_updates_stale` (3.0s) — dito
- CLI-Integration-Tests (0.1–0.25s) — Subprozess-Spawning, akzeptabel

---

## Lösung

### Ansatz: Shared Fixture mit `once_cell` / Setup-Reuse

**Kernidee:** Die teuren Schritte (Typst init, Build pipeline) sind identisch über viele Tests. Der Haupthebel liegt bei Kategorie A und B.

### Schritt 1: `BuildConfig::skip_pdf` Flag (Kategorie A+B)

Da die meisten build/place-Tests nur Layout-Logik testen, reicht es, PDF-Erzeugung zu überspringen. Die Typst-Tests (Kategorie B) bleiben wie sie sind – sie testen ja explizit die PDF-Pipeline und laufen parallel, 2–3s pro Test ist dort akzeptabel.

### Schritt 2: `BuildConfig::skip_pdf` Flag (Kategorie A)

Neues Bool in `BuildConfig` (Name: `skip_pdf`, default `false`). Wenn `true`, überspringt `build()` die komplette PDF-Erstellung (Typst-Kompilierung + Rasterisierung + Preview-Cache). Es wird nur die YAML-Datei mit dem Layout geschrieben.

**Effekt auf Kategorie A:** Die meisten build/place-Tests testen Solver-Logik, Layout-Zuordnung und State-Management – nicht die PDF-Ausgabe. Mit `skip_pdf: true` fällt der teure Typst+Render-Anteil (~2–3s) weg.

**Effekt auf Kategorie B:** Wird dadurch gelöst – die Typst-Tests, die tatsächlich PDF/Rendering testen, bleiben unverändert. Aber build/place-Tests, die nur Layout prüfen, brauchen kein Typst mehr.

### Schritt 3: Solver-Config in Tests anpassen (Kategorie A+C)

In Test-Helpers die `GaConfig` / `BookLayoutSolverConfig` aggressiv kürzen:
- `enable_local_search = false` — spart den teuersten Nachbearbeitungsschritt
- GA-Iterationen / Population reduzieren

```rust
// In test helpers:
mgr.state.config.book_layout_solver.enable_local_search = false;
mgr.state.config.page_layout_solver.generations = 50; // statt default 500
mgr.state.config.page_layout_solver.population = 20;  // statt default 100
```

**Effekt:** Solver-Anteil sinkt von ~1s auf <0.1s pro Test.

### Schritt 4: Große Solver-Tests hinter Feature-Flag (Kategorie C)

`test_no_split_needed` (12s) und `test_integration_large_instance_with_split` (>30s) behalten volle Problemgröße, werden aber mit `#[cfg(feature = "slow-tests")]` markiert.

```toml
# Cargo.toml
[features]
slow-tests = []
```

```rust
#[test]
#[cfg(feature = "slow-tests")]
fn test_no_split_needed() { ... }
```

- `cargo test` → schnell, ohne Langläufer
- `cargo test --features slow-tests` → alles inkl. großer Solver-Tests (CI-Release)

### Schritt 5: Cached Fixture mit LazyLock (place_test, build_test)

Für Tests die einen fertigen Build als Ausgangszustand brauchen (alle place_tests, einige build_tests): das teure Setup einmal ausführen und das Ergebnis-Verzeichnis read-only im Speicher halten. Vor jedem Test wird es in ein frisches TempDir kopiert (~1ms statt ~15s).

```rust
use std::sync::LazyLock;

static BUILT_PROJECT: LazyLock<TempDir> = LazyLock::new(|| {
    let temp = TempDir::new().unwrap();
    create_test_project_with_layout(&temp).unwrap();
    temp
});

fn fresh_project() -> (TempDir, PathBuf) {
    let fresh = TempDir::new().unwrap();
    copy_dir_recursive(BUILT_PROJECT.path(), fresh.path()).unwrap();
    (fresh, fresh.path().join("testplace"))
}
```

**Isolation:** Jeder Test mutiert nur sein eigenes TempDir. Das Original ist read-only.

**Effekt:** 8 place_tests + diverse build_tests sparen je ~15s Setup → einmalig ~15s + N×~1ms.
