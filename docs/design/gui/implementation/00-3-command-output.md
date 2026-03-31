# `CommandOutput<T>` + `render_pages()` — Fundament

> GUI-Kommentare sind rein informativ und werden hier nicht umgesetzt.

## Branch & Commits

- **Branch**: `feat/command-output` (von `main` abzweigen)
- **Author**: `EddyXorb`
- **Conventional Commits** nach jedem größeren Schritt

## 1 — `CommandOutput<T>` einführen

### StateManager::finish() → `Result<ProjectState>`

```rust
// Vorher:
pub fn finish(self, message: &str) -> Result<()>
pub fn finish_always(self, message: &str) -> Result<()>

// Nachher:
pub fn finish(self, message: &str) -> Result<ProjectState>
pub fn finish_always(self, message: &str) -> Result<ProjectState>
```

Intern: `self.state` wird am Ende ge-moved statt gedroppt.

### CommandOutput-Wrapper

```rust
// src/commands.rs
pub struct CommandOutput<T> {
    pub result: T,
    pub state: ProjectState,
}
```

### Betroffene Commands

Jeder Command, der `StateManager::finish()` aufruft, gibt jetzt den State weiter:

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
| `execute_weight()` | `Result<()>` | `Result<CommandOutput<()>>` |
| `project_new()` | `Result<NewResult>` | `Result<CommandOutput<NewResult>>` |
| `project_switch()` | `Result<()>` | `Result<CommandOutput<()>>` |

Read-Only Commands (`config`, `status`, `history`, `execute_info`) brauchen kein
`CommandOutput` — sie verändern keinen State.

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

### Tests

- Alle bestehenden Tests müssen weiterhin durchlaufen (Signaturänderung)
- Neuer Test: `CommandOutput` enthält korrekten State nach Command

### Commit-Plan

```
refactor(state): make StateManager::finish return ProjectState
feat(commands): introduce CommandOutput<T> wrapper
refactor(cli): adapt all CLI handlers to CommandOutput
test: verify existing tests pass with new signatures
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
fn compile_to_document(template_path: &Path) -> Result<typst::model::Document>
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
