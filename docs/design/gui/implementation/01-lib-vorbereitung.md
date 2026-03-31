# Phase 0: Lib-Vorbereitung

Reine Lib-Änderungen, kein GUI-Code. Ermöglicht der GUI, mit der Lib zu arbeiten.

## 0.1 — `CommandOutput<T>` einführen

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
`finish_internal` gibt `Result<ProjectState>` zurück.

### CommandOutput-Wrapper

```rust
// src/commands.rs (oder commands/mod.rs → commands.rs)
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
| `execute_mode()` | *(neu, siehe 00)* | `Result<CommandOutput<PageModeResult>>` |
| `execute_pos()` | *(neu, siehe 00)* | `Result<CommandOutput<PosResult>>` |
| `config_set()` | *(neu, siehe 00)* | `Result<CommandOutput<ConfigSetResult>>` |
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

## 0.2 — `render_pages()` in der Lib

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

### Implementierung

1. `compile_to_document()` extrahieren aus bestehendem `compile_to_bytes()`:
   - `compile_to_bytes` wird zu: `compile_to_document()` → `typst_pdf::pdf()`
   - Neue Funktion nutzt denselben `SimpleWorld`-Code
2. Für jede angefragte Seite: `typst_render::render(&document.pages[i], pixel_per_pt)`
3. Premultiplied → Straight Alpha Konvertierung:
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
4. Neue Dependency: `typst-render` (nicht feature-gated, da Lib-Funktion)

### Refactoring von compile_to_bytes

```rust
// Vorher: compile_to_bytes() → Vec<u8> (PDF)
// Nachher:
fn compile_to_document(template_path: &Path) -> Result<typst::model::Document>
fn compile_to_bytes(template_path: &Path) -> Result<Vec<u8>>  // nutzt compile_to_document
pub fn render_pages(...) -> Result<Vec<RenderedPage>>          // nutzt compile_to_document
```

## 0.3 — `PageMode` in LayoutPage

```rust
// src/dto_models/layout/layout_page.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPage {
    pub page: usize,
    #[serde(default, skip_serializing_if = "PageMode::is_auto")]
    pub mode: PageMode,
    pub photos: Vec<String>,
    pub slots: Vec<Slot>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PageMode {
    #[default]
    Auto,
    Manual,
}

impl PageMode {
    fn is_auto(&self) -> bool {
        *self == PageMode::Auto
    }
}
```

### Solver-Anpassung

Im inkrementellen Build (`src/commands/build.rs`): Manual-Seiten beim Rebuild überspringen.
Solver erhält nur Auto-Seiten.

### Abwärtskompatibilität

- `#[serde(default)]` → alte YAMLs ohne `mode` laden als Auto
- `skip_serializing_if` → Auto-Seiten schreiben kein `mode`-Feld → YAML bleibt gleich
