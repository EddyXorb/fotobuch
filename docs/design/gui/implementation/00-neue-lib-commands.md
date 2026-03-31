# Phase 0a: Neue Lib-Commands

Neue Commands die für die GUI (und die CLI) gebraucht werden.

## `config set` — Konfigurationswerte setzen

### CLI

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
```

### Lib

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

Implementierung:
1. `StateManager::open()`
2. Config als `serde_yaml::Value` laden
3. Key per Dot-Notation navigieren (`book.dpi` → `["book"]["dpi"]`)
4. Alten Wert merken, neuen Wert setzen (Typ-Erkennung: bool/number/string)
5. Zurück nach `ProjectConfig` deserialisieren (validiert automatisch)
6. `finish()` → State zurück

### GUI-Nutzung

Das Config-Panel ruft für jede Widget-Änderung `config_set()` im Background auf.
Kein Sonderweg — GUI und CLI nutzen denselben Codepfad.

---

## `page mode` — Seitenmodus umschalten

### CLI

```
fotobuch page mode <address> <auto|manual>
```

Beispiele:

```
fotobuch page mode 3 manual        # Seite 3 auf Manual
fotobuch page mode 3 auto          # Seite 3 zurück auf Auto
fotobuch page mode 0..5 manual     # Seiten 0-5 auf Manual
```

### Lib

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

Implementierung:
1. `StateManager::open()`
2. Für jede Seite im Adressbereich: `layout[i].mode = mode`
3. `finish()` → State zurück

Wechsel Manual→Auto markiert die Seite als dirty für den nächsten Build
(Solver optimiert sie beim nächsten `build` neu).

### GUI-Nutzung

Der [A|M]-Toggle pro Seite ruft `execute_mode()` im Background auf.

---

## `page pos` — Freie Positionierung (Manual Mode)

### CLI

Relatives Verschieben und Skalieren:

```
fotobuch page pos 4:2 --by -20,30             # -20mm x, +30mm y
fotobuch page pos 4:2 --by -20,30 --scale 1.5 # zusätzlich 1.5x skalieren
fotobuch page pos 4:2 --scale 0.8             # nur skalieren (um Slot-Zentrum)
```

Absolutes Positionieren:

```
fotobuch page pos 4:2 --at 100,50             # Slot-Ursprung auf (100mm, 50mm)
fotobuch page pos 4:2 --at 100,50 --scale 2.0 # absolut + skalieren
```

`--by` und `--at` schließen sich gegenseitig aus. `--scale` ist mit beiden kombinierbar.
Skalierung behält Aspect Ratio bei und skaliert um den Slot-Mittelpunkt.

Fehler wenn die Seite nicht im Manual-Mode ist.

### Lib

```rust
// src/commands/page/pos.rs
pub enum PosMode {
    Relative { dx_mm: f64, dy_mm: f64 },
    Absolute { x_mm: f64, y_mm: f64 },
}

pub struct PosConfig {
    pub mode: PosMode,
    pub scale: Option<f64>,  // None = keine Skalierung
}

pub struct PosResult {
    pub page: usize,
    pub slot: usize,
    pub old_slot: Slot,
    pub new_slot: Slot,
}

pub fn execute_pos(
    project_root: &Path,
    page: u32,
    slot: u32,
    config: &PosConfig,
) -> Result<CommandOutput<PosResult>>
```

Implementierung:
1. `StateManager::open()`
2. Prüfen: `layout[page].mode == Manual`, sonst Fehler
3. Slot lesen, neue Position berechnen:
   - Relative: `x_mm += dx`, `y_mm += dy`
   - Absolute: `x_mm = x`, `y_mm = y`
   - Scale: `width_mm *= s`, `height_mm *= s`, Mittelpunkt bleibt
4. `finish()` → State zurück

### GUI-Nutzung

Drag auf freie Fläche → berechnet Delta in mm → `execute_pos(Relative)` im Background.
Ecken-Drag → berechnet Scale-Faktor → `execute_pos(scale)` im Background.
