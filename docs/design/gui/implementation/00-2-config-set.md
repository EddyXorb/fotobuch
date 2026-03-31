# `config set` — Konfigurationswerte setzen

> GUI-Kommentare sind rein informativ und werden hier nicht umgesetzt.

## Branch & Commits

- **Branch**: `feat/config-set` (von `main` abzweigen)
- **Author**: `EddyXorb`
- **Conventional Commits** nach jedem größeren Schritt

## CLI

```
fotobuch config set <key> <value>
```

Dot-Notation wie bei `git config`, navigiert die YAML-Hierarchie:

```
fotobuch config set book.dpi 300
fotobuch config set book.gap_mm 3.5
fotobuch config set book.cover.active true
fotobuch config set page_layout_solver.mutation_rate 0.4
fotobuch config set book_layout_solver.page_target 24
fotobuch config set preview.show_filenames false
fotobuch config set book.cover.mode spread
```

## Lib

### Dateien

- `src/commands/config.rs` → wird zu `src/commands/config.rs` + `src/commands/config/set.rs`
  (bestehende `config()` und `render_config()` bleiben, `set` kommt dazu)
- CLI-Handler in `cli/cli/config.rs`

### Typen

```rust
// src/commands/config/set.rs
pub struct ConfigSetResult {
    pub key: String,
    pub old_value: String,
    pub new_value: String,
}

pub fn config_set(
    project_root: &Path,
    key: &str,
    value: &str,
) -> Result<CommandOutput<ConfigSetResult>>
```

### Implementierung

1. `StateManager::open()`
2. Config als `serde_yaml::Value` laden (`serde_yaml::to_value(&state.config)`)
3. Key per Dot-Notation navigieren:
   ```rust
   // "book.cover.active" → ["book", "cover", "active"]
   let parts: Vec<&str> = key.split('.').collect();
   let mut current = &mut value;
   for part in &parts[..parts.len()-1] {
       current = current.get_mut(part)?;
   }
   ```
4. Alten Wert merken (als String)
5. Neuen Wert setzen mit Typ-Erkennung:
   - `"true"` / `"false"` → `Value::Bool`
   - Parst als `f64` → `Value::Number`
   - Parst als `i64` → `Value::Number`
   - Sonst → `Value::String`
6. Zurück nach `ProjectConfig` deserialisieren → **validiert automatisch**
   (ungültige Werte / Keys → serde-Fehler)
7. `state.config = deserialized_config`
8. `finish()` → State zurück

### Fehlerbehandlung

- Key existiert nicht → Fehler beim Navigieren ("Unknown config key: book.foo")
- Wert hat falschen Typ → serde-Deserialisierung schlägt fehl
  ("Cannot parse 'abc' as value for book.dpi: expected number")
- Leerer Key → Fehler

### Typ-Erkennung (Reihenfolge)

```rust
fn parse_yaml_value(s: &str) -> Value {
    if s == "true" { return Value::Bool(true); }
    if s == "false" { return Value::Bool(false); }
    if let Ok(i) = s.parse::<i64>() { return Value::Number(i.into()); }
    if let Ok(f) = s.parse::<f64>() { return Value::Number(f.into()); }
    Value::String(s.to_string())
}
```

## Tests

### Unit Tests

- **Einfacher Wert**: `book.dpi` = `"300"` → `Value::Number(300)`
- **Verschachtelter Wert**: `book.cover.active` = `"true"` → `Value::Bool(true)`
- **Float**: `book.gap_mm` = `"3.5"` → `Value::Number(3.5)`
- **String**: `book.title` = `"Mein Buch"` → `Value::String("Mein Buch")`
- **Enum als String**: `book.cover.mode` = `"spread"` → valid nach Deserialisierung
- **Ungültiger Key**: `book.nonexistent` → Fehler
- **Ungültiger Wert**: `book.dpi` = `"abc"` → Fehler bei Deserialisierung
- **Roundtrip**: Wert setzen → Config laden → Wert prüfen

### Integrations-Test

- Projekt erstellen, `config set book.dpi 150`, `config` anzeigen → zeigt 150

## CLI-Output

```
$ fotobuch config set book.dpi 150
book.dpi: 300 → 150
```

## Commit-Plan

```
feat(config): add config set command for dot-notation config mutation
test(config): add unit tests for config set
feat(cli): add config set subcommand to CLI
```
