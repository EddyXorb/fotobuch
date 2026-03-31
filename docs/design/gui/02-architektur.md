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
- Ohne Argumente → GUI (wenn Feature an), sonst Help
- Mit Argumenten → immer CLI

## Modul-Struktur

```
src/
├── gui.rs              pub fn run() + FotobuchApp (feature-gated)
├── gui/
│   ├── state.rs        GuiState + DerivedState
│   ├── renderer.rs     Background-Rendering: Typst → Pixmap → egui::TextureHandle
│   ├── panels.rs       Re-exports der Panel-Module
│   ├── panels/
│   │   ├── main_view.rs  Scrollbare Seitenansicht mit Slot-Overlays
│   │   ├── photo_pool.rs Linkes Panel: Foto-Liste
│   │   ├── page_nav.rs   Rechtes Panel: Seiten-Thumbnails
│   │   └── config.rs     Config-Fenster (auto-generated aus serde_yaml::Value)
│   ├── interactions.rs Drag & Drop, Hotkeys, Selektion
│   └── toolbar.rs      Top-Bar + Statusbar
```

## Command-Rückgaben: `CommandOutput<T>`

### Problem

Commands haben jeweils eigene, sinnvolle Result-Structs (`BuildResult`, `AddResult`, ...).
Die GUI braucht nach jedem Command den aktuellen `ProjectState`.

### Lösung: Generischer Wrapper

```rust
// src/commands/mod.rs
pub struct CommandOutput<T> {
    pub result: T,
    pub state: ProjectState,
}
```

`StateManager::finish()` gibt den State zurück statt ihn zu droppen:

```rust
// StateManager
pub fn finish(self, message: &str) -> Result<ProjectState> {
    self.finish_internal(message, false)?;
    Ok(self.state)
}
```

Jeder Command gibt `Result<CommandOutput<XyzResult>>` zurück:

```rust
// Vorher:
pub fn build(...) -> Result<BuildResult>

// Nachher:
pub fn build(...) -> Result<CommandOutput<BuildResult>>
```

CLI-Handler ignorieren `.state`, GUI nutzt beides:

```rust
// CLI (ändert sich minimal)
let output = commands::build::build(...)?;
print_build_result(&output.result);

// GUI
let output = commands::build::build(...)?;
gui_state.apply(output.state);
// output.result.pages_rebuilt → nur diese Seiten re-rendern
```

### Dirty Pages aus Command Results

Die GUI muss nicht selbst diffen — die Commands liefern bereits, was sich geändert hat:

| Command | Affected Pages (aus Result) |
|---------|---------------------------|
| `build` | `pages_rebuilt`, `pages_swapped` |
| `rebuild` | `pages_rebuilt` (= BuildResult) |
| `place` | `pages_affected` |
| `remove` | `pages_affected` |
| `page swap/move` | betroffene Seiten-Indizes |
| `undo/redo` | alle Seiten (State komplett neu) |
| `config` | alle Seiten |

## State-Modell

### GuiState: Was die GUI hält

```rust
struct GuiState {
    // === Kern ===
    project_state: ProjectState,
    derived: DerivedState,

    // === Rendering ===
    page_textures: Vec<PageTexture>,
    dirty_pages: HashSet<usize>,
    building_pages: HashSet<usize>,

    // === UI ===
    selection: Selection,
    drag: Option<DragState>,
    zoom: f32,
    scroll_offset: f32,
    config_window_open: bool,
}
```

### DerivedState: Lookup-Caches

Wird einmal aus `ProjectState` berechnet und nach jedem Command-Update neu gebaut.

```rust
struct DerivedState {
    /// Foto-ID → PhotoFile (schneller Lookup für Tooltips, DPI-Berechnung etc.)
    photo_by_id: HashMap<String, PhotoFile>,

    /// Foto-ID → Gruppe
    group_of_photo: HashMap<String, String>,

    /// Foto-ID → (page, slot_index) — wo ist es platziert?
    placement_of_photo: HashMap<String, (usize, usize)>,

    /// Set aller platzierten Foto-IDs
    placed_photos: HashSet<String>,

    /// Alle unplatzierten Fotos (für den Foto-Pool)
    unplaced_photos: Vec<String>,

    /// Anzahl platzierter Fotos pro Gruppe (für Pool-Badges)
    placed_per_group: HashMap<String, usize>,
}

impl DerivedState {
    /// Komplett neu berechnen aus ProjectState
    fn rebuild(state: &ProjectState) -> Self {
        // Einmal über photos + layout iterieren,
        // alle Maps aufbauen
    }
}
```

### Update-Flow nach jedem Command

```rust
impl GuiState {
    fn apply(&mut self, output: CommandOutput<impl Any>) {
        self.project_state = output.state;
        self.derived = DerivedState::rebuild(&self.project_state);
        // dirty_pages aus dem jeweiligen Result setzen
    }
}
```

**Prinzip**: `DerivedState::rebuild()` ist die einzige Stelle, die Lookup-Strukturen baut.
Ein Aufruf, ein Codepfad, kein inkrementelles Patching.

### Kein StateManager in der GUI

Die GUI hält keinen `StateManager`. Der Lifecycle ist:

1. GUI ruft Lib-Command auf (z.B. `commands::page::swap(...)`)
2. Command öffnet intern `StateManager::open()`, mutiert, ruft `finish()` auf
3. `finish()` speichert YAML + Git-Commit, gibt `ProjectState` zurück
4. GUI erhält `CommandOutput<T>` mit dem neuen State
5. GUI ruft `DerivedState::rebuild()` auf, markiert dirty pages

Die GUI liest **nie** YAML-Dateien. Sie bekommt den State immer als Rückgabewert.

## Threading-Modell

### Grundregel: UI-Thread darf nie blocken

Alles was länger als ~1ms dauern kann, wird an einen Background-Thread delegiert.
Der UI-Thread macht **nur**:

- egui-Widgets zeichnen
- gecachte Texturen anzeigen
- Slot-Overlays berechnen (reine Geometrie, schnell)
- Input verarbeiten → Tasks in Channel schieben
- Result-Channel pollen (non-blocking)

### Was in den Background muss

| Operation | Grund |
|-----------|-------|
| Jeder Lib-Command (build, swap, move, add, ...) | StateManager öffnet YAML, schreibt, Git-Commit |
| `render_pages()` | Typst-Kompilierung + Rasterisierung |
| `DerivedState::rebuild()` | HashMap-Aufbau, bei vielen Fotos spürbar |
| Bild-Thumbnails laden (Foto-Pool) | Disk-I/O |
| Blur-Berechnung für "wird gebaut"-Effekt | CPU-intensiv |

### Was im UI-Thread bleiben kann

| Operation | Grund |
|-----------|-------|
| Slot-Hit-Test (Maus → welcher Slot?) | Einfache Rect-Checks |
| Selektion updaten | Nur Index-Sets ändern |
| Drag-State verwalten | Nur Position tracken |
| Zoom/Scroll updaten | Nur eine Zahl ändern |
| Texturen an egui übergeben | Pointer-Swap, kein Copy |

### Architektur: Ein Background-Thread mit Task-Queue

```
UI-Thread (60fps)                    Background-Thread
─────────────────                    ──────────────────
egui frame loop:                     loop {
  1. Widgets zeichnen                  task = rx.recv()
  2. Texturen anzeigen                 match task {
  3. Overlays                            Command(f) => {
  4. result_rx.try_recv()                  let out = f();
     → CommandDone? apply()                result_tx.send(CommandDone(out))
     → PageRendered? Textur swap           // + automatisch dirty pages rendern
     → DerivedReady? derived swap        }
     → ThumbnailReady? Pool update       RenderPages(pages, zoom) => {
  5. Input → task_tx.send(...)             let pix = render_pages(...);
                                           result_tx.send(PageRendered(pix))
                                         }
                                       }
                                     }
```

### Channels

```rust
// UI → Background
enum BackgroundTask {
    /// Lib-Command ausführen (build, swap, etc.)
    RunCommand(Box<dyn FnOnce() -> CommandOutputAny + Send>),
    /// Seiten rendern (nach Command oder Zoom-Änderung)
    RenderPages { pages: Vec<usize>, pixel_per_pt: f32 },
    /// Thumbnails für Foto-Pool laden
    LoadThumbnails(Vec<String>),  // photo_ids
}

// Background → UI
enum BackgroundResult {
    CommandDone { output: CommandOutputAny, dirty_pages: Vec<usize> },
    PageRendered(RenderedPage),
    DerivedReady(DerivedState),
    ThumbnailReady { photo_id: String, texture_data: Vec<u8> },
    Error(String),
}
```

### Ablauf einer User-Aktion (z.B. Swap)

```
1. User draggt Slot A auf Slot B
2. UI-Thread:
   - Setzt building_pages für betroffene Seiten
   - Zeigt Blur auf diesen Seiten
   - Sendet RunCommand(|| commands::page::swap(...)) an Background
3. Background-Thread:
   - Führt swap aus → CommandOutput<SwapResult>
   - Sendet CommandDone zurück
   - Baut DerivedState::rebuild() → sendet DerivedReady
   - Ruft render_pages() für dirty pages auf → sendet PageRendered
4. UI-Thread (nächste Frames):
   - try_recv() → CommandDone: project_state updaten
   - try_recv() → DerivedReady: derived state swappen
   - try_recv() → PageRendered: Textur swappen, Blur entfernen
```

### Konkurrenz-Handling

- Neue Aktion während laufendem Command: wird gequeued (Channel-Buffer)
- Neue Render-Anfrage während laufendem Render: alten abbrechen (Cancellation-Flag)
- UI zeigt immer den letzten bekannten State — auch wenn Background noch arbeitet

### Typst-Rendering: Lib-API statt GUI-Logik

Die Lib bietet eine neue Funktion in `output/typst.rs` an, analog zu `compile_preview`:

```rust
/// Rendert einzelne Seiten als RGBA-Pixmaps.
/// Die GUI ruft nur diese Funktion auf und kümmert sich nicht um Typst-Interna.
pub fn render_pages(
    project_root: &Path,
    project_name: &str,
    pages: &[usize],
    pixel_per_pt: f32,
) -> Result<Vec<RenderedPage>>

pub struct RenderedPage {
    pub page: usize,
    pub width: u32,
    pub height: u32,
    /// RGBA-Pixel, straight alpha (nicht premultiplied — fertig für egui)
    pub pixels: Vec<u8>,
}
```

Intern nutzt sie die bestehende `SimpleWorld` + `typst::compile()`, ersetzt aber
den `typst_pdf::pdf()`-Schritt durch `typst_render::render()` + Alpha-Konvertierung.

Vorteile:
- GUI hat keine Typst-Dependency (nur über Lib)
- Demultiply-Logik liegt zentral in der Lib
- Signatur ist simpel: Pfad rein, Pixel raus

### Zoom-Strategie

- GUI ruft `render_pages()` mit passendem `pixel_per_pt` für den aktuellen Zoom
- Zoom-Änderung > 2x: debounced Re-Render (~200ms)
- Dazwischen: GPU-Skalierung der vorhandenen Textur

## YAML-Erweiterung: Page Mode

```yaml
layout:
- page: 0
  mode: manual    # optional, fehlt = auto
  photos: [...]
  slots: [...]
```

```rust
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
