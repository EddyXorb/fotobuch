# Implementation Plan: `fotobuch remove`

Stand: 2026-03-08

## Überblick

Entfernt Fotos oder ganze Gruppen aus dem Projekt. Pflegt `photos` und `layout` konsistent.

## Abhängigkeiten

- `dto_models::ProjectState` load/save (vorhanden)
- `git::commit` / `git::is_git_repo` (vorhanden)
- `glob` crate für Pattern-Matching

## Pattern-Matching

Jedes Pattern wird in dieser Reihenfolge interpretiert:

1. **Exakter Gruppen-Name**: `state.photos` enthält eine Gruppe mit `group == pattern`
2. **Exakte Photo-ID**: `photo.id == pattern`
3. **Glob-Pattern**: via `glob::Pattern::matches()` gegen alle Photo-IDs

Mehrere Patterns werden mit OR verknüpft.

## Schritte in `commands/remove.rs`

1. **YAML laden**: `ProjectState::load("fotobuch.yaml")`
2. **Matching Photos sammeln**:
   - `matched_ids: HashSet<String>` — alle IDs die mindestens einem Pattern entsprechen
   - `matched_groups: Vec<String>` — Gruppen die komplett gematcht sind (alle Files gematcht oder Gruppe direkt angegeben)
3. **Aus `layout` entfernen** (immer, auch bei `--keep-files`):
   - Für jede `LayoutPage`: `photos.retain(|id| !matched_ids.contains(id))`
   - Korrespondierend `slots` neu aufbauen (nur Slots behalten deren Index-Position im gefilterten `photos` liegt)
   - Betroffene Seiten-Nummern sammeln
4. **Aus `photos` entfernen** (nur wenn `!keep_files`):
   - Gruppen-Files filtern, leere Gruppen entfernen
5. **YAML speichern**
6. **Git commit**: `remove: {n} photos` (oder `remove: group "{name}"`)

## Slot-Index-Anpassung

`photos` und `slots` sind index-gekoppelt. Beim Entfernen von Foto at index `i` muss `slots[i]` entfernt werden. Einfachste Umsetzung:

```rust
let new_photos: Vec<String> = page.photos.iter()
    .filter(|id| !matched_ids.contains(*id))
    .cloned().collect();
let new_slots: Vec<Slot> = page.photos.iter().zip(&page.slots)
    .filter(|(id, _)| !matched_ids.contains(*id))
    .map(|(_, slot)| slot.clone())
    .collect();
```

## Neue Crate

```toml
glob = "0.3"
```

## Konventionen

- **Conventional Commits**: Jeder Teilschritt bekommt einen eigenen Commit (z.B. `feat: implement pattern matching for remove`, `test: add unit tests for slot index adjustment`).
- **Tests vor jedem Commit**: `cargo test` muss fehlerfrei durchlaufen — kein Commit mit roten Tests.
- **Tests schreiben**: Für jeden neuen Teilschritt sind Unit-Tests (`#[cfg(test)]` inline) und Integrationstests (`tests/`) Pflicht.
- **`mod solver` unberührt**: Keine Änderungen in `src/solver/`. Alle Implementierungen spielen sich ausschließlich in `src/commands/` ab.
- **Dateigröße**: Überschreitet `remove.rs` ~300 Zeilen, in `remove/` aufteilen (z.B. `remove/pattern.rs`, `remove/layout.rs`).

## Tests

- Einzelnes Foto entfernen → aus `photos` und `layout` weg, Slots angepasst
- Ganze Gruppe entfernen → alle zugehörigen Layout-Einträge weg
- `--keep-files` → nur Layout bereinigt, `photos` bleibt
- Glob-Pattern: `"Urlaub/*"` matcht alle Fotos in Gruppe "Urlaub"
- Nicht-existentes Pattern → kein Fehler, 0 entfernt
