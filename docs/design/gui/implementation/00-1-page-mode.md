# `page mode` — Seitenmodus umschalten

> GUI-Kommentare sind rein informativ und werden hier nicht umgesetzt.

## Branch & Commits

- **Branch**: `feat/page-mode` (von `main` abzweigen)
- **Author**: `EddyXorb`
- **Conventional Commits** nach jedem größeren Schritt

## CLI

```
fotobuch page mode <address> <a|m|auto|manual>
```

`a` und `m` als Kurzform, `auto` und `manual` als Langform.

Beispiele:

```
fotobuch page mode 3 m           # Seite 3 auf Manual
fotobuch page mode 3 a           # Seite 3 zurück auf Auto
fotobuch page mode 0..5 m        # Seiten 0-5 auf Manual
fotobuch page mode 3 manual      # Langform
fotobuch page mode 3 auto        # Langform
```

## Lib

### Dateien

- `src/dto_models/layout/layout_page.rs` — `PageMode` Enum + Feld in `LayoutPage`
- `src/commands/page/mode.rs` — Implementierung
- `src/commands/build.rs` — Manual-Seiten beim Rebuild überspringen
- CLI-Handler in `cli/cli/page.rs`

### PageMode Enum

```rust
// src/dto_models/layout/layout_page.rs
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PageMode {
    #[default]
    Auto,
    Manual,
}

impl PageMode {
    pub fn is_auto(&self) -> bool {
        *self == PageMode::Auto
    }
}
```

`LayoutPage` bekommt das Feld:

```rust
#[serde(default, skip_serializing_if = "PageMode::is_auto")]
pub mode: PageMode,
```

### Command

```rust
// src/commands/page/mode.rs
pub struct PageModeResult {
    pub pages_changed: Vec<usize>,
    pub new_mode: PageMode,
}

pub fn execute_mode(
    project_root: &Path,
    pages: PageExpr,
    mode: PageMode,
) -> Result<CommandOutput<PageModeResult>>
```

### Implementierung

1. `StateManager::open()`
2. Für jede Seite im Adressbereich: `layout[i].mode = mode`
3. `finish()` → State zurück

Wechsel Manual→Auto markiert die Seite als dirty für den nächsten Build.

### Solver-Anpassung

Im inkrementellen Build: Manual-Seiten beim Rebuild überspringen.
Nur Seiten mit `mode == Auto` an den Solver übergeben.

### YAML-Abwärtskompatibilität

- `#[serde(default)]` → alte YAMLs ohne `mode` laden als Auto
- `skip_serializing_if` → Auto-Seiten schreiben kein `mode`-Feld

## CLI-Parsing

`PageMode` aus String parsen (für clap):

```rust
fn parse_page_mode(s: &str) -> Result<PageMode> {
    match s {
        "a" | "auto" => Ok(PageMode::Auto),
        "m" | "manual" => Ok(PageMode::Manual),
        _ => Err(anyhow!("Expected 'a', 'm', 'auto', or 'manual', got '{s}'")),
    }
}
```

## Tests

### Unit Tests

- **Set Manual**: Seite 3 Auto → `page mode 3 m` → Seite 3 Manual
- **Set Auto**: Seite 3 Manual → `page mode 3 a` → Seite 3 Auto
- **Range**: `page mode 0..2 m` → Seiten 0, 1, 2 Manual
- **Idempotent**: Seite schon Manual → nochmal `m` → kein Fehler, keine Änderung
- **Kurzformen**: `a` = `auto`, `m` = `manual`
- **YAML Roundtrip**: Manual-Seite serialisieren → `mode: manual` vorhanden
- **YAML Roundtrip**: Auto-Seite serialisieren → kein `mode`-Feld
- **YAML Rückwärts**: Altes YAML ohne `mode` → lädt als Auto

### Integrations-Test

- Build nach Manual→Auto Wechsel: Solver optimiert Seite neu

## Commit-Plan

```
feat(models): add PageMode enum to LayoutPage
feat(page): add mode subcommand for auto/manual toggle
feat(build): skip manual pages in incremental solver
feat(cli): add page mode subcommand to CLI
test(page): add unit tests for page mode
```
