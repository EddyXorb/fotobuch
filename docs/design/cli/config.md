# Implementation Plan: `fotobuch config`

Stand: 2026-03-08

## Überblick

Zeigt die vollständig aufgelöste Konfiguration — explizit gesetzte Werte und Defaults — als kommentiertes YAML.

## Abhängigkeiten

- `dto_models::ProjectState` load (vorhanden)
- `serde_yaml` (vorhanden)

## Problem: Default-Erkennung

`serde_yaml` kennt keine "was kam aus der Datei, was ist Default"-Unterscheidung. Lösung: **zwei Deserialisierungen**:

1. YAML als `serde_yaml::Value` laden → enthält nur explizit gesetzte Keys
2. Als `ProjectConfig` deserialisieren → enthält alle Werte mit Defaults

Durch Vergleich der Keys in der `serde_yaml::Value` vs. alle Felder der Struct kann die CLI-Schicht `# default` annotieren.

## Empfohlene Umsetzung

`config()` gibt `(ProjectConfig, serde_yaml::Value)` zurück:

```rust
pub fn config(project_root: &Path) -> Result<(ResolvedConfig, serde_yaml::Value)>
```

Die CLI-Schicht (`cli.rs`) rendert das YAML mit Annotationen:

- Für jeden Key der in `serde_yaml::Value` fehlt → Kommentar `# default` anhängen
- Ausgabe ist gültiges YAML (Kommentare am Zeilenende)

**Alternativ** (einfacher): `config()` serialisiert `ProjectConfig` zu YAML-String und vergleicht Key für Key mit der Raw-Value. Aufwand ähnlich.

## Defaults-Quelle

Alle Defaults via `#[serde(default = "...")]` in den Config-Structs — bereits vorhanden in `dto_models/config/`. Keine doppelte Definition nötig.

## CLI-Ausgabe Format

```yaml
config:
  book:
    page_width_mm: 420.0
    page_height_mm: 297.0
    bleed_mm: 3.0
    margin_mm: 10.0              # default
    gap_mm: 3.0                  # default
```

## Zusammenspiel mit `fotobuch new`

`new` schreibt eine vollständige YAML mit allen Feldern (keine Defaults nötig). `config` ist daher v.a. nützlich wenn Benutzer Teile der YAML gelöscht haben oder wissen wollen was einstellbar ist.

## Konventionen

- **Conventional Commits**: Jeder Teilschritt bekommt einen eigenen Commit (z.B. `feat: implement config command with default detection`, `test: add unit tests for default annotation`).
- **Tests vor jedem Commit**: `cargo test` muss fehlerfrei durchlaufen — kein Commit mit roten Tests.
- **Tests schreiben**: Für jeden neuen Teilschritt sind Unit-Tests (`#[cfg(test)]` inline) und Integrationstests (`tests/`) Pflicht.
- **`mod solver` unberührt**: Keine Änderungen in `src/solver/`. Alle Implementierungen spielen sich ausschließlich in `src/commands/config.rs` ab.
- **Dateigröße**: Bei >300 Zeilen `config.rs` in `config/` aufteilen.

## Tests

- Minimale YAML (nur Pflichtfelder) → alle anderen Felder als `# default` markiert
- Vollständige YAML → keine `# default`-Markierungen
- Ungültige YAML → Fehler
