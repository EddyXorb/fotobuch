# Implementation Plan: `fotobuch new`

Stand: 2026-03-08

## Überblick

Erstellt ein neues Fotobuch-Projekt: Verzeichnisstruktur, YAML, Git-Repo, Typst-Templates.

## Abhängigkeiten

- `std::process::Command` (git)
- `dto_models::ProjectState` / `ProjectConfig` / `BookConfig` (vorhanden)
- `serde_yaml` (vorhanden)
- Keine neuen Crates nötig

## Schritte in `commands/new.rs`

1. **Prüfen** ob `<parent>/<name>/` bereits existiert → Fehler falls ja
2. **Verzeichnisse anlegen**
   - `<name>/`
   - `<name>/.fotobuch/cache/preview/`
   - `<name>/.fotobuch/cache/final/`
3. **`fotobuch.yaml` schreiben** via `ProjectState::save()`
   - `config.book`: `title=name`, `page_width_mm`, `page_height_mm`, `bleed_mm` aus `NewConfig`
   - Alle anderen Felder: Defaults aus den structs
   - `photos: []`, `layout: []`
4. **`.gitignore` schreiben**
   ```
   .fotobuch/
   *.pdf
   ```
5. **Typst-Templates schreiben** (`fotobuch_preview.typ`, `fotobuch_final.typ`)
   - Statische Templates (fest, werden nie regeneriert) — Inhalt aus `workflow_and_cache_tasks.md §2.1`
   - Preview: `cache_prefix = ".fotobuch/cache/preview/"`
   - Final: `cache_prefix = ".fotobuch/cache/final/"`, kein Wasserzeichen
6. **Git initialisieren** via `Command`
   ```
   git init --initial-branch=fotobuch
   git add fotobuch.yaml .gitignore fotobuch_preview.typ fotobuch_final.typ
   git commit -m "new: {W}x{H}mm, {B}mm bleed"
   ```
   Falls git nicht im PATH: Fehler mit erklärender Meldung (git ist Pflicht, kein optionales Feature)

## Typst-Template-Inhalt

Beide Templates lesen `fotobuch.yaml` via `#yaml()` und rendern die Seiten:

```typst
#let data = yaml("fotobuch.yaml")
// ... Seiten aus data.layout iterieren
// Preview: Wasserzeichen via #place() + #rotate() + #text()
// Final: kein Wasserzeichen, keine Seitenzahlen
```

Die Templates werden als eingebettete Strings (`include_str!` oder Konstanten in `output.rs`) oder als Template-Dateien aus einem `templates/`-Verzeichnis geliefert.

**Empfehlung**: Zwei Konstanten in `src/output.rs`:
```rust
pub const PREVIEW_TEMPLATE: &str = include_str!("../templates/fotobuch_preview.typ");
pub const FINAL_TEMPLATE: &str = include_str!("../templates/fotobuch_final.typ");
```

## Neue Dateien

- `src/templates/fotobuch_preview.typ`
- `src/templates/fotobuch_final.typ`

## Signatur (bereits vorhanden, nur implementieren)

```rust
pub fn new(parent_dir: &Path, config: &NewConfig) -> Result<NewResult>
```

## Konventionen

- **Conventional Commits**: Jeder Teilschritt bekommt einen eigenen Commit (z.B. `feat: create project directory structure`, `test: add integration test for new command`).
- **Tests vor jedem Commit**: `cargo test` muss fehlerfrei durchlaufen — kein Commit mit roten Tests.
- **Tests schreiben**: Für jeden neuen Teilschritt sind Unit-Tests (`#[cfg(test)]` inline) und Integrationstests (`tests/`) Pflicht.
- **`mod solver` unberührt**: Keine Änderungen in `src/solver/`. Alle Implementierungen spielen sich ausschließlich in `src/commands/` und ggf. neuen Modulen (`cache/`, `output/`) ab.
- **Dateigröße**: Überschreitet eine Datei ~300 Zeilen, wird sie in einen gleichnamigen Unterordner aufgeteilt (z.B. `new.rs` → `new/` mit `new/git.rs`, `new/yaml.rs` etc.), analog zur bestehenden Modulstruktur im Projekt.

## Tests

- Tempdir: Verzeichnisstruktur prüfen
- YAML laden und Werte verifizieren
- `.gitignore` vorhanden
- Git-Repo initialisiert (`git log` hat einen Commit)
- Fehler bei existierendem Verzeichnis
