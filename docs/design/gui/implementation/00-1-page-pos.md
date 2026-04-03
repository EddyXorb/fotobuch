# `page pos` — Freie Slot-Positionierung (Manual Mode)

> GUI-Kommentare sind rein informativ und werden hier nicht umgesetzt.

## Branch & Commits

- **Branch**: `feat/page-pos` (von `main` abzweigen)
- **Author**: `EddyXorb`
- **Conventional Commits** nach jedem größeren Schritt

## CLI

```
fotobuch page pos <address> [--by dx,dy] [--at x,y] [--scale s]
```

Beispiele:

```
fotobuch page pos 4:2 --by -20,30              # Slot 2: -20mm x, +30mm y
fotobuch page pos 4:2 --by -20,30 --scale 1.5  # verschieben + skalieren
fotobuch page pos 4:2 --scale 0.8              # nur skalieren
fotobuch page pos 4:2..5 --by -20,30           # Slots 2-5 gemeinsam verschieben
fotobuch page pos 4:2 --at 100,50              # Slot-Ursprung auf (100mm, 50mm)
fotobuch page pos 4:2 --at 100,50 --scale 2.0  # absolut + skalieren
```

Regeln:
- `--by` und `--at` schließen sich gegenseitig aus
- `--scale` ist mit beiden kombinierbar
- Mindestens eins von `--by`, `--at`, `--scale` muss angegeben sein
- Akzeptiert Slot-Ranges und Komma-Listen (SlotExpr)
- **Fehler** wenn die Seite nicht im Manual-Mode ist

### Skalierungsverhalten

Skalierung geht nach **rechts unten**: der Ursprung (x_mm, y_mm) des Slots
bleibt unverändert, nur width_mm und height_mm werden multipliziert.
Aspect Ratio bleibt erhalten.

```
Vor Scale 1.5:              Nach Scale 1.5:
┌──────┐                    ┌──────────────┐
│(x,y) │                    │(x,y)         │
│      │                    │              │
└──────┘                    │              │
                            │              │
                            └──────────────┘
```

## Lib

### Dateien

- `src/commands/page/pos.rs` — Implementierung
- CLI-Handler in `cli/cli/page.rs` — Subcommand-Parsing + Aufruf

### Typen

```rust
// src/commands/page/pos.rs
pub enum PosMode {
    Relative { dx_mm: f64, dy_mm: f64 },
    Absolute { x_mm: f64, y_mm: f64 },
}

pub struct PosConfig {
    pub position: Option<PosMode>,  // None wenn nur --scale
    pub scale: Option<f64>,
}

pub struct PosResult {
    pub page: usize,
    pub slots_changed: Vec<SlotChange>,
}

pub struct SlotChange {
    pub slot: usize,
    pub old: Slot,
    pub new: Slot,
}

pub fn execute_pos(
    project_root: &Path,
    page: u32,
    slots: SlotExpr,
    config: &PosConfig,
) -> Result<CommandOutput<PosResult>>
```

### Implementierung

1. `StateManager::open()`
2. Prüfen: `layout[page].mode == Manual`, sonst Fehler
3. Für jeden Slot in der SlotExpr:
   - Relative: `x_mm += dx`, `y_mm += dy`
   - Absolute: `x_mm = x`, `y_mm = y`
   - Scale: `width_mm *= s`, `height_mm *= s` (Ursprung bleibt)
4. `finish()` → State zurück

### Abhängigkeit

Benötigt `PageMode` in `LayoutPage` (wird in `feat/page-mode` eingeführt).
Entweder `feat/page-mode` zuerst mergen, oder `PageMode` hier minimal einführen
und in `feat/page-mode` den Rest ergänzen.

## Tests

### Unit Tests (`src/commands/page/pos.rs`)

- **Relativer Move**: Slot bei (10, 20) + `--by 5,-3` → (15, 17)
- **Absoluter Move**: Slot bei (10, 20) + `--at 50,60` → (50, 60)
- **Scale**: Slot (10, 20, w=100, h=50) + `--scale 2.0` → (10, 20, w=200, h=100)
- **Kombination**: `--by 5,5 --scale 0.5` → erst verschieben, dann skalieren
- **Multi-Slot**: Slots 2..4 + `--by 10,10` → alle drei verschoben
- **Fehler: nicht Manual**: Seite im Auto-Mode → Fehler
- **Fehler: Slot out of range**: Slot-Index > Anzahl Slots → Fehler
- **Fehler: kein Argument**: weder --by noch --at noch --scale → Fehler

### Integrations-Test

- Projekt mit Manual-Seite erstellen, `page pos` ausführen, YAML prüfen

## Commit-Plan

```
feat(page): add pos subcommand for manual slot positioning
test(page): add unit tests for page pos
feat(cli): add page pos subcommand to CLI
```
