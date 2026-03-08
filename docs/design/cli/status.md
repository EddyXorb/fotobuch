# Implementation Plan: `fotobuch status`

Stand: 2026-03-08

## Überblick

Zeigt Projektstatus und erkennt Änderungen seit dem letzten Build via Struct-Diff mit Git-Snapshot.

## Abhängigkeiten

- `dto_models::ProjectState` load (vorhanden)
- `std::process::Command` für `git show HEAD:fotobuch.yaml`

## Änderungserkennung

```rust
// Letzten committeten Zustand laden:
let output = Command::new("git")
    .args(["show", "HEAD:fotobuch.yaml"])
    .current_dir(project_root)
    .output()?;
let committed = ProjectState::from_yaml_bytes(&output.stdout)?;  // neue Hilfsmethode
let current = ProjectState::load("fotobuch.yaml")?;
```

Fallback wenn kein Git / kein Commit: Status ohne Diff-Info anzeigen.

### Rebuild nötig wenn (pro Seite):

- `layout[i].photos` hat andere Länge als committed
- Ein Photo-ID wurde durch ein anderes mit **anderem Ratio** ersetzt
  - Ratio = `width_px as f64 / height_px as f64` aus `state.photos`
  - Toleranz: 5% → `(ratio_a - ratio_b).abs() / ratio_a > 0.05`
- `area_weight` eines platzierten Fotos hat sich geändert

### Nur Swap (kein Rebuild) wenn:

- Photos mit Ratio-kompatiblem Tausch (Toleranz 5%) — innerhalb oder seitenübergreifend

## Konsistenzprüfungen

```
placed_ids = alle IDs in layout[].photos (als HashSet)
all_ids    = alle IDs in photos[].files  (als HashSet)

unplaced   = all_ids - placed_ids  → Info, kein Fehler
orphaned   = placed_ids - all_ids  → Warnung
```

## Hilfsmethode für Ratio

```rust
fn photo_ratio(state: &ProjectState, id: &str) -> Option<f64> {
    state.photos.iter()
        .flat_map(|g| &g.files)
        .find(|f| f.id == id)
        .map(|f| f.width_px as f64 / f.height_px as f64)
}
```

## Neue Methode in `ProjectState`

```rust
pub fn from_yaml_bytes(bytes: &[u8]) -> Result<Self>
```

## Keine neuen Crates nötig

## Konventionen

- **Conventional Commits**: Jeder Teilschritt bekommt einen eigenen Commit (z.B. `feat: implement git snapshot diff for status`, `test: add unit tests for ratio comparison`).
- **Tests vor jedem Commit**: `cargo test` muss fehlerfrei durchlaufen — kein Commit mit roten Tests.
- **Tests schreiben**: Für jeden neuen Teilschritt sind Unit-Tests (`#[cfg(test)]` inline) und Integrationstests (`tests/`) Pflicht.
- **`mod solver` unberührt**: Keine Änderungen in `src/solver/`. Alle Implementierungen spielen sich ausschließlich in `src/commands/` ab.
- **Dateigröße**: Überschreitet `status.rs` ~300 Zeilen, in `status/` aufteilen (z.B. `status/diff.rs`, `status/consistency.rs`).

## Tests

- Leeres Layout → "No layout yet"
- Unplaced Photos werden korrekt erkannt
- Orphaned Placements werden korrekt erkannt
- Ratio-kompatibler Tausch → kein Rebuild nötig
- Ratio-inkompatibler Tausch → Rebuild nötig
- Ohne Git: Status funktioniert ohne Änderungserkennung
- Detail-View `status 5`: SlotInfo pro Photo mit Ratio und Swap-Gruppe
