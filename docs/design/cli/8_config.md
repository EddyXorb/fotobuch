# Implementation Plan: `fotobuch config`

Stand: 2026-03-08

## Überblick

Zeigt die vollständig aufgelöste Konfiguration — explizit gesetzte Werte und Defaults — als kommentiertes YAML. Rein lesend — verändert nichts, kein Git.

## Abhängigkeiten

- `dto_models::ProjectState` load (vorhanden)
- `serde_yaml` (vorhanden)

**Keine neuen Crates. Kein Git.**

---

## Problem: Default-Erkennung

`serde_yaml` kennt keine "was kam aus der Datei, was ist Default"-Unterscheidung. Lösung: **zwei Deserialisierungen**:

1. YAML als `serde_yaml::Value` laden → enthält nur explizit gesetzte Keys
2. Als `ProjectConfig` deserialisieren → enthält alle Werte mit Defaults

Durch Vergleich der Keys in der `serde_yaml::Value` vs. alle Felder der Struct kann die Annotation `# default` gesetzt werden.

---

## Signaturen

### `src/commands/config.rs`

```rust
use anyhow::{Context, Result};
use std::path::Path;

use crate::dto_models::ProjectConfig;

/// Ergebnis: aufgelöste Config + Raw-Value für Default-Erkennung.
pub struct ConfigResult {
    pub resolved: ProjectConfig,
    pub raw: serde_yaml::Value,
}

/// Lädt die Config mit aufgelösten Defaults und dem Raw-YAML für Annotation.
pub fn config(project_root: &Path) -> Result<ConfigResult> {
    let yaml_path = project_root.join("fotobuch.yaml");
    let contents = std::fs::read_to_string(&yaml_path)
        .with_context(|| format!("Failed to read {}", yaml_path.display()))?;

    // 1. Raw-Value: nur explizit gesetzte Keys
    let full_value: serde_yaml::Value = serde_yaml::from_str(&contents)?;
    let raw_config = full_value
        .get("config")
        .cloned()
        .unwrap_or(serde_yaml::Value::Mapping(Default::default()));

    // 2. Vollständig deserialisiert mit Defaults
    let state = crate::dto_models::ProjectState::load(&yaml_path)?;

    Ok(ConfigResult {
        resolved: state.config,
        raw: raw_config,
    })
}
```

### Annotation-Logik (CLI-Schicht)

Die CLI-Schicht (`cli.rs` oder ähnlich) rendert das YAML mit `# default` Markierungen:

```rust
/// Rendert die aufgelöste Config als YAML mit `# default` Annotationen.
/// Felder die nicht in raw_config vorkommen werden als Default markiert.
pub fn render_config(result: &ConfigResult) -> Result<String> {
    // 1. resolved Config serialisieren
    let resolved_yaml = serde_yaml::to_string(&result.resolved)?;

    // 2. Zeilenweise annotieren
    let lines: Vec<String> = resolved_yaml
        .lines()
        .map(|line| annotate_line(line, &result.raw))
        .collect();

    Ok(lines.join("\n"))
}

/// Prüft ob ein YAML-Key in der Raw-Value vorhanden ist.
/// Navigiert verschachtelte Mappings via Key-Pfad.
///
/// Ansatz: Aus der Zeile den Key-Pfad extrahieren (Einrückung = Tiefe),
/// dann in raw_config nachschauen ob der Key existiert.
fn annotate_line(line: &str, raw_config: &serde_yaml::Value) -> String {
    // Zeilen ohne Key (Listen, Kommentare, leer) → unverändert
    // Zeilen mit Key → Pfad aufbauen, in raw_config suchen
    // Nicht gefunden → "  # default" anhängen
}
```

**Alternativer Ansatz** (robuster): Statt zeilenbasierter Annotation den `serde_yaml::Value`-Baum direkt traversieren und annotiertes YAML selbst generieren:

```rust
/// Traversiert resolved und raw parallel, generiert annotiertes YAML.
fn render_annotated(
    resolved: &serde_yaml::Value,
    raw: &serde_yaml::Value,
    indent: usize,
    output: &mut String,
) {
    match resolved {
        serde_yaml::Value::Mapping(map) => {
            let raw_map = raw.as_mapping();
            for (key, value) in map {
                let key_str = key.as_str().unwrap_or("");
                let is_default = raw_map
                    .map(|m| !m.contains_key(key))
                    .unwrap_or(true);

                if value.is_mapping() {
                    // Nested: rekursiv, kein # default auf der Mapping-Zeile selbst
                    write_indent(output, indent);
                    output.push_str(&format!("{key_str}:\n"));
                    let child_raw = raw_map
                        .and_then(|m| m.get(key))
                        .cloned()
                        .unwrap_or(serde_yaml::Value::Mapping(Default::default()));
                    render_annotated(value, &child_raw, indent + 2, output);
                } else {
                    // Leaf: Wert + ggf. # default
                    write_indent(output, indent);
                    let val_str = format_scalar(value);
                    if is_default {
                        output.push_str(&format!("{key_str}: {val_str:<24}# default\n"));
                    } else {
                        output.push_str(&format!("{key_str}: {val_str}\n"));
                    }
                }
            }
        }
        _ => {
            // Scalar am Top-Level (unwahrscheinlich für Config)
            write_indent(output, indent);
            output.push_str(&format!("{}\n", format_scalar(resolved)));
        }
    }
}

fn format_scalar(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Null => "null".to_string(),
        _ => serde_yaml::to_string(value).unwrap_or_default().trim().to_string(),
    }
}

fn write_indent(output: &mut String, indent: usize) {
    for _ in 0..indent {
        output.push(' ');
    }
}
```

**Empfehlung**: Den rekursiven `render_annotated`-Ansatz verwenden — er ist robuster als zeilenbasiertes Parsing, weil er die Baumstruktur direkt nutzt statt YAML-Zeilen zu parsen.

---

## CLI-Ausgabe Format

```yaml
config:
  book:
    title: Mein Fotobuch
    page_width_mm: 420.0
    page_height_mm: 297.0
    bleed_mm: 3.0
    margin_mm: 10.0              # default
    gap_mm: 5.0                  # default
    bleed_threshold_mm: 3.0      # default
  page_layout_solver:
    seed: 42                     # default
    population_size: 200         # default
    max_generations: 1000        # default
    ...
  preview:
    show_filenames: true         # default
    show_page_numbers: true      # default
    max_preview_px: 800          # default
  book_layout_solver:
    page_target: 20              # default
    ...
```

Die Ausgabe ist gültiges YAML (Kommentare stören serde nicht) — copy-paste in `fotobuch.yaml` wenn man einen Default überschreiben will.

---

## Zusammenspiel mit `fotobuch new`

`new` schreibt eine vollständige YAML mit allen Feldern. `config` ist v.a. nützlich wenn der Benutzer Teile der YAML gelöscht hat oder wissen will was einstellbar ist.

---

## Defaults-Quelle

Alle Defaults via `#[serde(default = "...")]` in den Config-Structs — bereits vorhanden in `dto_models/config/`. Keine doppelte Definition nötig.

---

## Implementierungsreihenfolge

| #   | Schritt | Abhängig von |
| --- | ------- | ------------ |
| 1 | `config()` — zwei Deserialisierungen, `ConfigResult` zurückgeben | — |
| 2 | `render_annotated` — rekursive YAML-Generierung mit `# default` | 1 |
| 3 | CLI-Integration: `fotobuch config` Subcommand verdrahten | 1, 2 |

Jeder Schritt = ein Commit.

## Konventionen

- **Conventional Commits**: z.B. `feat: implement config command with default detection`, `feat: implement annotated YAML rendering`
- **Tests**: Unit-Tests + Integrationstests für jeden Schritt
- **`mod solver` unberührt**: Alle Implementierungen in `src/commands/config.rs` und der CLI-Schicht

## Tests

| Test | Prüft |
| ---- | ----- |
| Minimale YAML (nur Pflichtfelder) → alle optionalen Felder als `# default` markiert | Default-Erkennung |
| Vollständige YAML → keine `# default`-Markierungen | Explizite Werte |
| Teilweise überschriebene Defaults → nur fehlende als `# default` | Mischung |
| Verschachtelte Defaults (z.B. `weights` in `page_layout_solver`) → korrekt annotiert | Rekursive Traversierung |
| Ungültige YAML → sinnvoller Fehler | Fehlerbehandlung |
| Ausgabe ist gültiges YAML (re-parse möglich) | Format-Korrektheit |
