# GUI Architektur

## Crate-Layout: GUI getrennt von der Lib

Die GUI lebt **außerhalb von `src/`** als eigener Binary-Target, analog zur bestehenden CLI. `src/` bleibt UI-agnostisch — weder `eframe` noch `egui` tauchen im Dep-Graph der Library auf.

```
fotobuch/
├── src/     Lib (Kernlogik, UI-frei)
├── cli/     CLI-Binary  → `fotobuch`
└── gui/     GUI-Binary  → `fotobuch-gui`
```

### Cargo.toml

```toml
[[bin]]
name = "fotobuch"
path = "cli/main.rs"

[[bin]]
name = "fotobuch-gui"
path = "gui/main.rs"
required-features = ["gui"]

[features]
gui = ["dep:eframe", "dep:egui"]

[dependencies]
eframe = { version = "…", optional = true, default-features = false, features = ["default_fonts", "glow"] }
egui   = { version = "…", optional = true }
# typst-render und image liegen in der Lib (nicht optional) — `render_pages()` ist Lib-API.
```

- `cargo build` → nur `fotobuch` (CLI). Keine eframe/egui-Deps.
- `cargo build --features gui` → `fotobuch` + `fotobuch-gui`.
- `cargo run --bin fotobuch-gui --features gui` → startet die GUI.

**Kein Dispatch aus dem CLI-Binary.** GUI und CLI sind zwei unabhängige Executables — das CLI-Binary bleibt auch im Release frei von eframe/egui. Der ursprünglich geplante „keine Args → GUI"-Switch entfällt zugunsten dieser sauberen Trennung.

## Modul-Struktur

```
gui/
├── main.rs           eframe::run_native, lädt ProjectState, startet Worker
├── app.rs            FotobuchApp (impl eframe::App), Input-Handling
├── app/
│   ├── geometry.rs     Slot-Rect-Mapping, Hit-Test
│   ├── overlay.rs      Timings-Overlay (F2)
│   ├── statusbar.rs    Statusbar unten
│   ├── toolbar.rs      Top-Bar
│   ├── panels.rs       Panel-Modul-Wurzel (deklariert main_view, photo_pool, page_nav)
│   └── panels/
│       ├── main_view.rs    Scrollbare Seitenansicht mit Slot-Overlays
│       ├── photo_pool.rs   Linkes Panel: Foto-Liste
│       ├── page_nav.rs     Rechtes Panel: Seiten-Thumbnails
│       └── config.rs       Config-Fenster (auto-generated aus serde_yaml::Value)
├── state.rs          GuiState
├── state/
│   ├── derived.rs      DerivedState (Lookup-Caches)
│   ├── selection.rs    Selection-Enum
│   └── timings.rs      Per-Frame-Timings
├── background.rs     Worker-Thread + Rayon-Pool + TypstWorld
└── task.rs           BackgroundTask / BackgroundResult / CommandDone
```

- `gui/main.rs` ist der Binary-Root und deklariert die Submodule direkt (`mod app; mod state; …`).
- UI-Rendering lebt komplett unter `app/`, State-Logik unter `state/`.
- Konvention aus `.claude/CLAUDE.md`: kein `mod.rs`, stattdessen `foo.rs` neben `foo/`.
- Alle Seitenindizes 0-basiert.

## Command-Rückgaben: `CommandOutput<T>`

### Problem

Commands haben jeweils eigene, sinnvolle Result-Structs (`BuildResult`, `AddResult`, ...).
Die GUI braucht nach jedem Command den aktuellen `ProjectState` — aber nur,
wenn der Command ihn tatsächlich geändert hat.

### Lösung: Generischer Wrapper (bereits implementiert in `src/commands.rs`)

```rust
pub struct CommandOutput<T> {
    pub result: T,
    /// `Some(state)` wenn der Command committet hat, `None` bei No-Ops
    /// (read-only Commands oder Commands ohne effektive Änderung).
    pub changed_state: Option<ProjectState>,
}
```

`StateManager::finish()` gibt `Result<Option<ProjectState>>` zurück: `None`
wenn der Diff leer ist (kein Commit), `Some(state)` nach erfolgreichem Commit.
Jeder Command reicht dieses Option durch.

CLI-Handler ignorieren `.changed_state`, GUI nutzt beides:

```rust
// CLI (unverändert: liest nur .result)
let output = commands::build::build(...)?;
print_build_result(&output.result);

// GUI: No-Ops = kein DerivedState-Rebuild, kein Re-Render
let output = commands::build::build(...)?;
if let Some(state) = output.changed_state {
    gui_state.apply(state, output.result.pages_rebuilt);
}
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
    page_textures: Vec<Option<PageTexture>>,  // index-coupled mit layout[]
    dirty_pages: HashSet<usize>,
    building_pages: HashSet<usize>,
    /// Pro-Seite monoton wachsender Counter. Eine Neuanforderung für Seite `p`
    /// inkrementiert **nur** `render_epochs[p]` — laufende Renderings für
    /// unberührte Seiten bleiben gültig und ihr Ergebnis wird akzeptiert.
    /// Index-coupled mit `page_textures`.
    render_epochs: Vec<u64>,

    // === UI ===
    selection: Selection,
    drag: Option<DragState>,
    zoom: f32,
    scroll_offset: f32,
    config_window_open: bool,
    toasts: VecDeque<Toast>,            // Fehler/Warnungen aus dem Background
}

struct PageTexture {
    handle: egui::TextureHandle,
    /// Seitenmaße in Pt — benötigt für Slot-Overlay-Geometrie.
    width_pt: f32,
    height_pt: f32,
    /// Epoch, mit der diese Textur gerendert wurde.
    rendered_at: u64,
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
    /// Wird aufgerufen, sobald der Background-Thread `CommandDone` liefert.
    /// `dirty_pages` werden vom jeweiligen Command-Result extrahiert
    /// (siehe Tabelle unten) und an den Background-Renderer weitergereicht.
    fn apply(&mut self, new_state: ProjectState, dirty_pages: Vec<usize>) {
        self.project_state = new_state;
        self.derived = DerivedState::rebuild(&self.project_state);
        self.dirty_pages.extend(dirty_pages);
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
    /// Lib-Command ausführen. Closure kapselt alle Argumente und gibt
    /// ein konkretes `CommandDone`-Variant zurück — kein `dyn Any` nötig.
    RunCommand(Box<dyn FnOnce() -> CommandDone + Send>),
    /// Seiten rendern (nach Command oder Zoom-Änderung).
    /// `epoch` stammt vom UI-Thread (`render_epoch.fetch_add(1, …)`).
    RenderPages { pages: Vec<usize>, pixel_per_pt: f32, epoch: u64 },
    /// Thumbnails für Foto-Pool laden (bounded: nur sichtbarer Bereich).
    LoadThumbnails(Vec<String>),
}

/// Ein Enum pro Command-Gruppe, damit der UI-Thread typsicher branchen kann
/// ohne Downcasting. Nutzt direkt `CommandOutput<T>` aus der Lib —
/// `changed_state` ist `None` bei No-Ops, dann kein Rebuild.
enum CommandDone {
    LayoutChange(CommandOutput<PageMoveResult>),  // Swap, Move, Unplace, Split, Combine
    Build(CommandOutput<BuildResult>),
    Place(CommandOutput<PlaceResult>),
    Undo(CommandOutput<UndoResult>),
    // …weitere Varianten nach Bedarf
    Failed(String),
}

// Background → UI
enum BackgroundResult {
    CommandDone(CommandDone),
    /// Rendered page + die Epoch, zu der es gehört. UI verwirft, wenn
    /// `epoch < render_epochs[page.page]` (stale gegenüber einer neueren
    /// Anforderung für **dieselbe** Seite).
    PageRendered { page: RenderedPage, epoch: u64 },
    ThumbnailReady { photo_id: String, rgba: Vec<u8>, width: u32, height: u32 },
    Toast(Toast),  // Info/Warning/Error, UI puffert in VecDeque
}
```

`CommandDone::dirty_pages()` kapselt die Mapping-Tabelle (Build → `pages_rebuilt`, LayoutChange → `pages_modified ∪ pages_inserted` (bei Deletion: alle Seiten), Undo → alle Seiten).

### Ablauf einer User-Aktion (z.B. Swap)

```
1. User draggt Slot A auf Slot B
2. UI-Thread:
   - Setzt building_pages für betroffene Seiten (→ Blur/Spinner-Overlay)
   - Für jede betroffene Seite `p`: `render_epochs[p] += 1` — invalidiert
     nur in-flight Renderings für **diese** Seiten. Unberührte Seiten
     behalten ihre laufenden Renderings.
   - Sendet RunCommand(|| commands::page::swap(...)) an Background
3. Background-Thread:
   - Führt swap aus → Result<CommandOutput<PageMoveResult>>
   - Sendet CommandDone { result, state } zurück
   - Wenn state = Some: sendet sofort RenderPages-Task an sich selbst
     (die dirty pages kommen aus extract_dirty_pages); jede Seite in
     diesem Task trägt die aktuelle `render_epochs[p]` als Tag.
4. UI-Thread (nächste Frames, ~immer < 16ms):
   - try_recv() → CommandDone(Move): gui_state.apply() — ruft intern
     DerivedState::rebuild() synchron auf (einige Millisekunden bei
     typischen Projekten; ggf. später off-thread ziehen).
   - try_recv() → PageRendered { page, epoch }: wenn
     `epoch ≥ render_epochs[page.page]`, Textur swappen und
     building_pages bereinigen, sonst verwerfen (stale).
```

`DerivedState::rebuild()` wird bewusst im UI-Thread aufgerufen, **nicht**
im Worker: nur so ist die sichtbare Reihenfolge garantiert (state + derived
werden atomar gegenüber der UI ersetzt). Bei Projekten mit >10k Fotos ggf.
später in den Worker auslagern und via `BackgroundResult::DerivedReady`
austauschen — YAGNI bis Messwerte das rechtfertigen.

### Konkurrenz-Handling

- **Commands**: serialisiert im Background (ein Worker-Thread, FIFO-Channel).
  StateManager-interner Git-Commit ist sequentiell, Parallelisierung bringt nichts.
- **Rendering**: Epoch-basiert **pro Seite**. Jede Render-Anfrage trägt
  `(page, epoch = render_epochs[page])`. Bei Zoom-Änderung werden nur die
  Epochen sichtbarer Seiten inkrementiert; bei einem Command nur die dirty
  pages aus `extract_dirty_pages`. UI verwirft `PageRendered`-Ergebnisse,
  deren Epoch kleiner ist als die aktuelle Seiten-Epoch — aber noch laufende
  Renderings für unberührte Seiten liefern ihr Ergebnis ganz normal ab.
  Der Typst-Compile läuft nicht preemptiv ab, aber stale Ergebnisse werden
  nie angezeigt.
- **Thumbnails**: eigener Channel, niedrigere Priorität. Beim Scroll werden
  die Epochen der Fotos invalidiert, die aus dem Viewport rausrutschen.
- UI zeigt immer den letzten bekannten State — auch wenn Background noch arbeitet.

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

### Initial Load

Beim Start in `gui/main.rs`:

1. Projekt ermitteln (CLI-Arg oder letztes Projekt aus Config) — synchron.
2. `state_manager::load_project_state(project_root, project_name)` — synchron,
   ein einmaliges YAML-Read ist akzeptabel.
3. `DerivedState::rebuild(&state)` — synchron beim ersten Aufruf (kleiner State).
4. `eframe::run_native(...)` startet; der Worker-Thread rendert alle Seiten
   im Hintergrund mit `pixel_per_pt = 1.0`. Solange noch keine Textur da ist,
   zeigt die Hauptansicht graue Platzhalter-Rechtecke im korrekten Seitenformat
   (aus `config.book.width/height` berechnet) inkl. Spinner.
5. Fehler beim Laden → Startup-Modal mit Fehlermeldung, dann App-Exit.

## YAML-Erweiterung: Page Mode (bereits implementiert)

```yaml
layout:
- page: 0
  mode: manual    # optional, fehlt = auto
  photos: [...]
  slots: [...]
```

```rust
// src/dto_models/layout/layout_page.rs
pub struct LayoutPage {
    pub page: usize,
    pub photos: Vec<String>,
    pub slots: Vec<Slot>,
    /// Abwärtskompatibilität kommt über `#[serde(default)]` + `#[default]`
    /// auf PageMode — fehlendes `mode` im YAML → `Auto`.
    #[serde(default, skip_serializing_if = "is_auto")]
    pub mode: PageMode,
}

#[derive(Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PageMode { #[default] Auto, Manual }

fn is_auto(mode: &PageMode) -> bool { *mode == PageMode::Auto }
```

Zwei Zustände, zwei Bedeutungen — keine `Option`-Schicht obendrauf. Der
GUI-Toggle `[A|M]` setzt direkt `PageMode::Auto` oder `PageMode::Manual`;
beim Serialisieren verschwindet `Auto` aus dem YAML, damit Bestands-Files
unverändert bleiben.
