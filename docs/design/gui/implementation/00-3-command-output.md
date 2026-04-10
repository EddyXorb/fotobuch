# `CommandOutput<T>` + `render_pages()` — Fundament

> GUI-Kommentare sind rein informativ und werden hier nicht umgesetzt.

## Branch & Commits

- **Branch**: `feat/command-output` (von `main` abzweigen)
- **Author**: `EddyXorb`
- **Conventional Commits** nach jedem größeren Schritt

## 1 — `CommandOutput<T>` einführen

### StateManager::finish() → `Result<Option<ProjectState>>`

```rust
// Vorher:
pub fn finish(self, message: &str) -> Result<()>
pub fn finish_always(self, message: &str) -> Result<()>

// Nachher:
pub fn finish(self, message: &str) -> Result<Option<ProjectState>>
pub fn finish_always(self, message: &str) -> Result<Option<ProjectState>>
```

- `None` → kein Commit (State unverändert)
- `Some(state)` → Commit erstellt, State hat sich geändert

Intern: `self.state` wird nur ge-moved wenn tatsächlich committed wird.

### CommandOutput-Wrapper

```rust
// src/commands.rs
pub struct CommandOutput<T> {
    pub result: T,
    pub changed_state: Option<ProjectState>,
}
```

`changed_state: None` bedeutet: State unverändert nach diesem Command.
`changed_state: Some(s)` bedeutet: State hat sich geändert, hier ist der neue State.

### Alle Commands geben CommandOutput zurück

Einheitliche API — kein Unterschied mehr zwischen read-only und write Commands:

| Command | Vorher | Nachher |
|---------|--------|---------|
| `build()` | `Result<BuildResult>` | `Result<CommandOutput<BuildResult>>` |
| `rebuild()` | `Result<BuildResult>` | `Result<CommandOutput<BuildResult>>` |
| `add()` | `Result<AddResult>` | `Result<CommandOutput<AddResult>>` |
| `place()` | `Result<PlaceResult>` | `Result<CommandOutput<PlaceResult>>` |
| `remove()` | `Result<RemoveResult>` | `Result<CommandOutput<RemoveResult>>` |
| `undo()` | `Result<UndoResult>` | `Result<CommandOutput<UndoResult>>` |
| `redo()` | `Result<UndoResult>` | `Result<CommandOutput<UndoResult>>` |
| `execute_move()` | `Result<PageMoveResult, PageMoveError>` | `Result<CommandOutput<PageMoveResult>, PageMoveError>` |
| `execute_unplace()` | `Result<PageMoveResult, PageMoveError>` | `Result<CommandOutput<PageMoveResult>, PageMoveError>` |
| `execute_split()` | `Result<PageMoveResult, PageMoveError>` | `Result<CommandOutput<PageMoveResult>, PageMoveError>` |
| `execute_combine()` | `Result<PageMoveResult, PageMoveError>` | `Result<CommandOutput<PageMoveResult>, PageMoveError>` |
| `execute_weight()` | `Result<(), PageMoveError>` | `Result<CommandOutput<()>, PageMoveError>` |
| `execute_mode()` | `Result<PageModeResult, PageMoveError>` | `Result<CommandOutput<PageModeResult>, PageMoveError>` |
| `execute_pos()` | `Result<PosResult, PageMoveError>` | `Result<CommandOutput<PosResult>, PageMoveError>` |
| `config_set()` | `Result<ConfigSetResult>` | `Result<CommandOutput<ConfigSetResult>>` |
| `project_new()` | `Result<NewResult>` | `Result<CommandOutput<NewResult>>` |
| `project_switch()` | `Result<()>` | `Result<CommandOutput<()>>` |
| `config()` | `Result<ConfigResult>` | `Result<CommandOutput<ConfigResult>>` |
| `status()` | `Result<StatusReport>` | `Result<CommandOutput<StatusReport>>` |
| `history()` | `Result<Vec<HistoryEntry>>` | `Result<CommandOutput<Vec<HistoryEntry>>>` |
| `execute_info()` | `Result<PageInfo, PageMoveError>` | `Result<CommandOutput<PageInfo>, PageMoveError>` |
| `project_list()` | `Result<Vec<String>>` | `Result<CommandOutput<Vec<String>>>` |

Read-only Commands geben immer `changed_state: None` zurück — kein extra YAML-Read.

### Anpassung der CLI-Handler

Minimal — jeder Handler destrukturiert nur `.result`:

```rust
// Vorher:
let result = commands::build::build(root, &config)?;
print_build_result(&result);

// Nachher:
let output = commands::build::build(root, &config)?;
print_build_result(&output.result);
```

### GUI-Nutzung

```rust
let output = command::build(...)?;
if let Some(new_state) = output.changed_state {
    gui.update_state(new_state);
}
```

### Commit-Plan

```
refactor(state): make StateManager::finish return Option<ProjectState>
feat(commands): introduce CommandOutput<T> with changed_state for all commands
refactor(cli): adapt all CLI handlers to CommandOutput
```

## 2 — `render_pages()` in der Lib

Neue Funktion in `src/output/typst.rs`:

```rust
pub struct RenderedPage {
    pub page: usize,
    pub width: u32,
    pub height: u32,
    /// RGBA-Pixel, straight alpha (fertig für egui)
    pub pixels: Vec<u8>,
}

pub fn render_pages(
    project_root: &Path,
    project_name: &str,
    pages: &[usize],
    pixel_per_pt: f32,
) -> Result<Vec<RenderedPage>>
```

### Refactoring von compile_to_bytes

```rust
// Vorher: compile_to_bytes() → Vec<u8> (PDF)
// Nachher:
fn compile_to_document(template_path: &Path) -> Result<PagedDocument>
fn compile_to_bytes(template_path: &Path) -> Result<Vec<u8>>  // nutzt compile_to_document
pub fn render_pages(...) -> Result<Vec<RenderedPage>>          // nutzt compile_to_document
```

### Premultiplied → Straight Alpha

```rust
for pixel in pixmap.pixels_mut().chunks_exact_mut(4) {
    let a = pixel[3] as f32 / 255.0;
    if a > 0.0 {
        pixel[0] = (pixel[0] as f32 / a).min(255.0) as u8;
        pixel[1] = (pixel[1] as f32 / a).min(255.0) as u8;
        pixel[2] = (pixel[2] as f32 / a).min(255.0) as u8;
    }
}
```

Neue Dependency: `typst-render` (nicht feature-gated, da Lib-Funktion)

### Tests

- **Render einer einfachen Seite**: Template mit Text → Pixmap hat korrekte Dimensionen
- **Seitenauswahl**: 3-Seiten-Dokument, nur Seite 1 rendern → eine RenderedPage zurück
- **Alpha-Konvertierung**: Pixel mit bekanntem premultiplied Wert → korrekt demultiplied
- **Bestehende compile-Tests** laufen weiterhin (compile_to_bytes nutzt compile_to_document)

### Commit-Plan

```
refactor(typst): extract compile_to_document from compile_to_bytes
feat(typst): add render_pages for raster page rendering
test(typst): add tests for render_pages and alpha conversion
```
