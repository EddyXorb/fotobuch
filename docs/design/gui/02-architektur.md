# GUI Architektur

## Feature-Gate und Binary-Integration

```toml
# Cargo.toml
[features]
gui = ["dep:eframe", "dep:egui", "dep:typst-render", "dep:image"]
```

```rust
// cli/main.rs (Pseudocode)
fn main() {
    if std::env::args().len() > 1 {
        // CLI-Modus: wie bisher
        let cli = Cli::parse();
        cli.command.execute()
    } else {
        #[cfg(feature = "gui")]
        gui::run();
        #[cfg(not(feature = "gui"))]
        Cli::command().print_help();
    }
}
```

- `cargo build` → nur CLI
- `cargo build --features gui` → CLI + GUI
- Ohne Argumente + mit GUI-Feature → GUI startet
- Mit Argumenten → immer CLI, egal ob GUI-Feature an

## Modul-Struktur

```
src/gui/              (feature-gated: #[cfg(feature = "gui")])
├── mod.rs            pub fn run() + FotobuchApp struct
├── state.rs          GuiState: wraps ProjectState + UI-spezifischer State
├── renderer.rs       Background-Rendering: Typst → Pixmap → egui::TextureHandle
├── panels/
│   ├── mod.rs
│   ├── main_view.rs  Scrollbare Seitenansicht mit Slot-Overlays
│   ├── photo_pool.rs Linkes Panel: Foto-Liste
│   ├── page_nav.rs   Rechtes Panel: Seiten-Thumbnails
│   └── config.rs     Config-Fenster (auto-generated aus serde_yaml::Value)
├── interactions.rs   Drag & Drop, Hotkeys, Selektion
└── toolbar.rs        Top-Bar + Statusbar
```

~10 Dateien. Jede fokussiert auf eine Aufgabe.

## State-Modell

```rust
struct GuiState {
    // === Kern-State (aus Lib) ===
    project_state: ProjectState,       // config + photos + layout
    state_manager: StateManager,       // persistenz + git + undo/redo

    // === Rendering-Cache ===
    page_textures: Vec<PageTexture>,   // eine Textur pro Seite
    dirty_pages: HashSet<usize>,       // Seiten die neu gerendert werden müssen
    building_pages: HashSet<usize>,    // Seiten die gerade gebaut werden

    // === UI-State ===
    selected_slot: Option<(usize, usize)>,  // (page, slot)
    drag: Option<DragState>,
    zoom: f32,
    scroll_offset: f32,
    config_window_open: bool,
}

struct PageTexture {
    texture: egui::TextureHandle,
    /// Blur-Version für "wird gerade gebaut"-Zustand
    blurred: Option<egui::TextureHandle>,
    render_scale: f32,          // bei welchem Zoom gerendert
}

enum DragState {
    Slot { page: usize, slot: usize, offset: Vec2 },
    Page { page: usize },
    PhotoFromPool { photo_id: String },
}
```

### Prinzip: ProjectState ist die Single Source of Truth

- Alle Änderungen gehen durch `ProjectState`
- GUI-State ist nur View-Logik (Zoom, Selektion, Drag)
- Config-Änderungen → `ProjectState.config` mutieren → dirty pages
- Undo/Redo = StateManager (Git-basiert, wie CLI)

## Rendering-Pipeline

```
                    UI-Thread (60fps)
                    ─────────────────
                    egui frame:
                      1. Input verarbeiten
                      2. Gecachte Texturen zeichnen
                      3. Slot-Overlays zeichnen
                      4. Channel pollen
                         ↓ neue Textur?  → swap in, blur weg
                         ↓ state change? → dirty pages markieren
                                           ↓
                              ┌─────────────────────────────────┐
                              │     Background-Thread           │
                              │                                 │
    dirty pages ─────────→    │  1. Solver (nur Auto-Seiten)    │
                              │  2. YAML schreiben              │
                              │  3. Typst kompilieren           │
                              │  4. typst-render → Pixmap       │
                              │  5. Pixmap demultiply alpha     │
                              │  6. → Channel → UI-Thread       │
                              └─────────────────────────────────┘
```

### Kommunikation: Channels

```rust
// UI → Background
enum RenderRequest {
    RebuildPages(Vec<usize>),
    FullBuild,
    ReleaseBuild,
}

// Background → UI
enum RenderResult {
    PageReady { page: usize, pixmap: Vec<u8>, width: u32, height: u32 },
    BuildComplete { result: BuildResult },
    Error(String),
}
```

### Typst-Rendering: Einzelseiten

Wichtig: Typst kompiliert das ganze Dokument, aber `typst-render` kann **einzelne Seiten** rendern:

```rust
let document = typst::compile(&world).output?;
// Nur Seite i rendern:
let pixmap = typst_render::render(&document.pages[i], pixel_per_pt);
// pixmap ist premultiplied → demultiply für egui
```

### Zoom-Strategie

- Texturen werden bei bestimmtem Zoom-Level gerendert
- Bei Zoom-Änderung > 2x: neu rendern (debounced, ~200ms)
- Zwischen-Zooms: GPU-Skalierung der vorhandenen Textur (schnell, leicht unscharf)
- Ergebnis: sofortiges visuelles Feedback, scharfes Bild nach kurzer Verzögerung

## Integration mit der Lib

Die GUI ruft **dieselben Funktionen** wie die CLI:

| GUI-Aktion | Lib-Funktion |
|------------|-------------|
| Swap Slots | `commands::page::swap()` |
| Move Foto | `commands::page::move_photo()` |
| Unplace | `commands::unplace()` |
| Build | `commands::build::build()` |
| Release | `commands::build::build(release=true)` |
| Undo/Redo | `StateManager::undo()`/`redo()` |
| Config ändern | `ProjectState.config` direkt mutieren |
| Add Photos | `commands::add::add()` |

Kein Duplizieren von Logik. Die GUI ist nur eine andere Eingabemethode.

## YAML-Erweiterung: Page Mode

```yaml
layout:
- page: 0
  mode: manual    # optional, fehlt = auto
  photos: [...]
  slots: [...]
- page: 1
  # kein mode-Feld = auto (implizit)
  photos: [...]
  slots: [...]
```

```rust
// dto_models/layout/layout_page.rs
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
```

Abwärtskompatibel: bestehende YAMLs ohne `mode` funktionieren (= Auto).
