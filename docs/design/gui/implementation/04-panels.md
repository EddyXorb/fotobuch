# Phase 4: Panels

**Ziel**: Foto-Pool links, Seiten-Navigation rechts, Config-Fenster. Danach ist das
Drei-Panel-Layout aus `01-ux-konzept.md` vollständig; Cross-Page und neue Seiten
bleiben Phase 5, Manual-Mode Phase 6.

**Voraussetzung**: Phase 3 abgeschlossen. `BackgroundTask`/`BackgroundResult`,
`PendingCommand`-Dispatch, Drag/Swap/Move und Undo/Redo laufen. Rayon-Pool und
persistente `TypstWorld` sind vorhanden (`gui/background.rs`).

---

## Überblick der Änderungen

| Bereich | Neue / geänderte Dateien |
|---|---|
| Layout-Migration | `gui/app.rs`, `gui/app/widgets.rs`, `gui/app/widgets/page_nav.rs` (neu), `gui/app/widgets/photo_pool.rs` (neu), `gui/app/widgets/config_window.rs` (neu) |
| Seiten-Nav       | `gui/app/widgets/page_nav.rs`, `gui/state.rs` (Thumb-Textur-Feld), `gui/task.rs` (`PageRendered` erweitert), `gui/background.rs` (Downsample-Postprocess) |
| Foto-Pool        | `gui/app/widgets/photo_pool.rs`, `gui/state/pool.rs` (neu), `gui/state.rs`, `gui/task.rs` (`LoadPhotoThumbnails`, `PhotoThumbnailReady`), `gui/thumbnail.rs` (neu, Decoder), `gui/background.rs` (Thumb-Worker) |
| Place-Dispatch   | `gui/app/pending.rs`, `gui/app/input_handler.rs`, `gui/app/widgets/toolbar.rs` |
| Config-Panel     | `gui/app/widgets/config_window.rs`, `gui/state/config_panel.rs` (neu), `gui/task.rs` (`ConfigSet`), `gui/background.rs` |

Keine neuen Lib-Commands — `place` (Filter + `into_page`), `execute_move`
(Page↔Page), `config_set` (dot-notation) decken alles ab.

---

## 4.0 — Vorbereitungen: Layout, State, Tasks

Ein einziger Commit, der die Gerüste stellt. Danach erst die drei Feature-Commits.

### 4.0.1 Zwei neue Widgets + Config-Window im `FotobuchApp::ui`

Bisheriger Stand in `gui/app.rs`: `widgets::toolbar::draw` (Panel::top, liefert
`cmds`), `widgets::statusbar::draw` (Panel::bottom), `widgets::central_panel::draw`
(CentralPanel). Jedes Widget rahmt sein Panel intern — diesem Muster folgen auch
die neuen Widgets.

```rust
// gui/app.rs — ui()
let mut cmds = widgets::toolbar::draw(ui, &mut self.state);
widgets::statusbar::draw(ui, &self.state);
widgets::photo_pool::draw(ui, &mut self.state, &mut cmds);
widgets::page_nav::draw(ui, &mut self.state, &mut cmds);
widgets::central_panel::draw(ui, &mut self.state);

if self.state.config_panel.open {
    widgets::config_window::show(&ctx, &mut self.state, &mut cmds);
}
```

Reihenfolge: TopBottom-Panels zuerst (Toolbar, Statusbar), dann Side-Panels,
zuletzt CentralPanel — so verlangt es egui.

Jedes neue Widget rahmt sein Panel intern, analog zur Toolbar:

```rust
// widgets/photo_pool.rs
pub fn draw(ui: &mut egui::Ui, state: &mut GuiState, cmds: &mut HashSet<PendingCommand>) {
    egui::SidePanel::left("photo_pool")
        .resizable(true).min_width(220.0).max_width(400.0).default_width(260.0)
        .show_inside(ui, |ui| show(ui, state, cmds));
}

// widgets/page_nav.rs
pub fn draw(ui: &mut egui::Ui, state: &mut GuiState, cmds: &mut HashSet<PendingCommand>) {
    egui::SidePanel::right("page_nav")
        .resizable(true).min_width(100.0).max_width(200.0).default_width(120.0)
        .show_inside(ui, |ui| show(ui, state, cmds));
}
```

In 4.0 enthält `show` nur einen Platzhalter-Label; echter Inhalt folgt in 4.1/4.2.

### 4.0.2 `GuiState` erweitern

```rust
pub struct GuiState {
    // … existing fields …

    /// Downsample-Textur pro Seite, index-coupled mit page_textures.
    pub page_thumb_textures: Vec<Option<TextureHandle>>,

    /// Foto-Thumbnails (256 px längste Kante).
    pub photo_thumbs: HashMap<String, TextureHandle>,
    /// IDs, für die ein LoadPhotoThumbnails-Task unterwegs ist — verhindert Doppel-Requests.
    pub photo_thumb_in_flight: HashSet<String>,
    /// FIFO für Hintergrund-Prefetch (Fotos außerhalb des Viewports).
    pub photo_thumb_prefetch: VecDeque<String>,

    /// Pool-Selektion: analog zu Selection, aber auf Foto-IDs.
    pub pool_selection: PoolSelection,
    /// Drag, der im Pool startet und auf eine Page gedroppt wird.
    pub pool_drag: PoolDragState,

    /// Zielseite, zu der beim nächsten Frame gescrollt werden soll (Nav-Klick).
    pub scroll_to_page: Option<usize>,

    /// Einmalig pro Frame: wenn `true`, zeichnet `draw_page` ein rotes Overlay
    /// über alle Slots mit mehrfach platziertem Foto. Wird am Ende von `ui()`
    /// wieder auf `false` gesetzt (Trigger: Hover über Statusbar-Duplikatsegment).
    pub highlight_duplicates: bool,

    /// State des Config-Fensters.
    pub config_panel: ConfigPanelState,
}
```

`resize_page_vecs` muss `page_thumb_textures` mit resizen (einfacher Zweizeiler).

### 4.0.3 `BackgroundTask` / `BackgroundResult` erweitern

```rust
pub enum BackgroundTask {
    // … bestehende Varianten …

    /// Foto-Thumbnails laden (Pool-Panel, Quell-Dateien auf Disk). UI sendet
    /// pro Frame die sichtbaren IDs zuerst, Rest tröpfelnd in Chunks — FIFO
    /// reicht als Priorisierung. Abgrenzung: Seiten-Thumbs im Nav werden als
    /// Downsample im `PageRendered`-Pfad transportiert, nicht hier.
    LoadPhotoThumbnails { items: Vec<(String /* id */, PathBuf /* source */)> },

    /// Place selektierte Fotos — entweder auf eine konkrete Zielseite
    /// (`dst_page = Some(_)`, z.B. aus Pool→Page-Drag) oder ohne Zielseite
    /// (`None` → Lib verteilt timestamp-basiert über alle Seiten). Slot-Auswahl
    /// übernimmt der Solver beim nächsten Build.
    PlacePhotos { photo_ids: Vec<String>, dst_page: Option<usize>, pixel_per_pt: f32 },

    /// `config set key value` im Background.
    ConfigSet { key: String, value: String, pixel_per_pt: f32 },
}

pub enum BackgroundResult {
    PageRendered {
        page: RenderedPage,
        /// Downsample (längste Kante ~120 px). Selber Frame, selber Task —
        /// UI hebt full + thumb atomar an.
        thumb: RenderedPage,
        rasterize_duration: Duration,
        compile_duration: Duration,
    },
    // … CommandDone/CommandFailed/Error …

    PhotoThumbnailReady {
        id: String,
        width: u32,
        height: u32,
        pixels: Vec<u8>, // straight-alpha RGBA
    },
}
```

### 4.0.4 Neue `PendingCommand`-Varianten

```rust
pub enum PendingCommand {
    // … Swap/Move/Undo/Redo …
    Place       { photo_ids: Vec<String>, dst_page: Option<usize> },
    PageSwap    { left: usize, right: usize },   // Nav-Drag Seite↔Seite
    ConfigSet   { key: String, value: String },
}
```

`dispatch_commands` mappt jede Variante 1:1 auf den passenden `BackgroundTask`.
`handle_input` emittiert sie.

### 4.0.5 Commit

`feat(gui): panels scaffolding — side panels, state fields, new tasks`

Tests: `state::resize_page_vecs_also_resizes_thumb_vec`.

---

## 4.1 — Seiten-Navigation (rechts)

### 4.1.1 Thumb-Downsample als Teil des Renders

Im Rayon-Task in `gui/background.rs::render_pages` direkt nach `rasterize_page`:

```rust
let full = rasterize_page(&doc, page, pixel_per_pt)?;
let thumb = downsample(&full, NAV_THUMB_MAX_EDGE_PX); // pure Funktion, testbar
tx.send(BackgroundResult::PageRendered { page: full, thumb, rasterize_duration, compile_duration });
```

`downsample(src: &RenderedPage, max_edge_px: u32) -> RenderedPage` lebt in
`src/output/render.rs` (Lib, damit UI-frei). Implementierung: `image::RgbaImage`
aus `src.pixels` wrappen, `image::imageops::resize(..., FilterType::Triangle)` mit
Zielmaßen so, dass längste Kante = `max_edge_px`, Aspect ratio erhalten.
Konstante `NAV_THUMB_MAX_EDGE_PX = 120` in `gui/app/widgets/page_nav.rs`.

Tests (Lib): `downsample_keeps_aspect_ratio`, `downsample_clamps_to_max_edge`,
`downsample_noop_when_already_small` (Input kleiner als max → unverändert durch).

### 4.1.2 Upload der Thumb-Textur

`state::apply_rendered` erhält zusätzlich `thumb: RenderedPage` und lädt beide
Texturen auf einmal:

```rust
pub fn apply_rendered(state: &mut GuiState, ctx: &Context, full: RenderedPage, thumb: RenderedPage) {
    // … wie bisher für full …
    let thumb_img = ColorImage::from_rgba_unmultiplied([thumb.width as _, thumb.height as _], &thumb.pixels);
    let thumb_tex = ctx.load_texture(format!("page_thumb_{}", thumb.page), thumb_img, TextureOptions::LINEAR);
    if let Some(s) = state.page_thumb_textures.get_mut(thumb.page) { *s = Some(thumb_tex); }
}
```

### 4.1.3 UI: Thumbnail-Leiste

`gui/app/widgets/page_nav.rs`:

- `ScrollArea::vertical()` innerhalb des SidePanels.
- Pro Seite `i`: vertikales Group-Layout mit
  - Thumb-Image (`fit_to_exact_size([panel_width - 20, _])`), Aspect aus Textur.
  - Label `P{i}` + Badge `[A]` / `[M]` (aus `project_state.layout[i].mode`).
  - Dünner Rahmen, der bei `i == hovered_page_for_nav` farbig wird (Drop-Target,
    siehe 4.1.5).
- Klick auf einen Thumb → `state.scroll_to_page = Some(i)` und
  `state.selection.clear()` (Seitenwechsel verwirft Slot-Selektion, wie in
  Phase 2 definiert).
- Falls noch kein Thumb vorhanden: graue Box mit korrektem Seitenverhältnis
  (aus `config.book.page_width/height`).

### 4.1.4 Scroll-to-page

In `gui/app/widgets/central_panel/draw_pages.rs`:

- `draw_page` gibt optional die `page_rect` zurück.
- Nach der Schleife: wenn `state.scroll_to_page == Some(i)` und wir `page_rect_of_i`
  haben, `ui.scroll_to_rect(page_rect_of_i, Some(Align::TOP))` und
  `state.scroll_to_page = None` (einmalig).

### 4.1.5 Nav-Drag: Seite ↔ Seite tauschen

Neuer Drag-Zustand in `gui/state/drag.rs`:

```rust
pub enum NavDragState {
    Idle,
    Dragging { src_page: usize },
}
```

`GuiState::nav_drag: NavDragState` (neu).

Input-Handling (in `page_nav::draw` direkt an der Thumb-Response):

- `response.drag_started()` mit sekundärem Button (konsistent zum Central-Drag)
  → `nav_drag = Dragging { src_page: i }`.
- `response.hovered()` während Drag → `hovered_page_for_nav = Some(i)` (lokal im
  Panel), Rahmenfarbe ändern.
- `response.drag_released()` mit gültigem Ziel ≠ Quelle →
  `cmds.insert(PendingCommand::PageSwap { left: src, right: dst })`,
  `page_dirty` für beide auf `true`.

Background-Worker (`gui/background.rs`) für `PageSwap`:

```rust
PageMoveCmd::Swap {
    left:  Src::Pages(PagesExpr::single(left as u32)),
    right: DstSwap::Pages(PagesExpr::single(right as u32)),
}
```

Gleiche `run_page_command`-Helfer wie in Phase 3 nutzen — kein neuer Code-Pfad.

### 4.1.6 Nav als Drop-Target für Central-Slot-Drags (Cross-Page Move)

Bereits existierende `DragState::Dragging` aus dem Central-Panel: wenn der Drop
auf einem Nav-Thumb landet, behandeln wir das wie „Move auf Seite P".

Realisierung: `page_nav::draw` prüft bei jedem Thumb, ob das Central-Drag aktiv
ist (`state.drag != DragState::Idle`) **und** die Thumb-`Response` hovered ist.
Wenn ja, setzt es `state.hovered_page = Some(i)` — der bestehende
`handle_drag_complete` in `input_handler.rs` greift dann **unverändert** und
emittiert `PendingCommand::Move { src_page, src_slots, dst_page: i }`.

Einziger neuer Code hier: die Hover-Detection + blaues Overlay auf dem Thumb
während ein Central-Drag läuft. Keine weitere Logik.

### 4.1.7 Commits

1. `feat(lib): image downsample helper for page thumbnails`
2. `feat(gui): page nav panel with thumbnails + click-to-scroll`
3. `feat(gui): page nav drag swaps pages, accepts slot drops for cross-page move`

Tests:
- `downsample_*` (Lib, s. o.).
- `page_nav::scroll_to_page_rect_clears_request_once` (logisch, pure).
- `input_handler::cross_page_move_when_drop_on_nav_thumb` (erweitert den
  bestehenden Test für `dispatch_move`, mit `hovered_page` aus Nav).

---

## 4.2 — Foto-Pool (links) — Datenmodell & Thumbnail-Worker

### 4.2.1 `PoolSelection` (neu, `gui/state/pool.rs`)

```rust
pub enum PoolSelection {
    None,
    Some { ids: BTreeSet<String>, anchor: String },
}

impl PoolSelection {
    pub fn single(id: String) -> Self;
    pub fn toggle(&mut self, id: String);                      // Ctrl+Klick
    pub fn range_to(&mut self, id: String, order: &[String]);  // Shift+Klick (braucht
                                                                // Reihenfolge für Range)
    pub fn clear(&mut self);
    pub fn is_selected(&self, id: &str) -> bool;
}
```

`order` ist die aktuelle Anzeigereihenfolge aller Fotos im Pool — reingereicht
durch den Aufrufer. Damit bleibt `PoolSelection` testbar ohne UI-State.

Tests (rein logisch, keine egui-Abhängigkeit):
- `single_replaces_previous`, `toggle_adds_then_removes`,
  `range_fills_forward_and_backward`, `clear_resets`.

### 4.2.2 Pool-Drag-State

```rust
// gui/state/drag.rs
pub enum PoolDragState {
    Idle,
    Dragging { photo_ids: Vec<String> },  // Snapshot der Selektion beim drag_started
}
```

`photo_ids` ist **bewusst ein Snapshot** — die UI darf die Selektion während
des Drags weiter verändern, der Drag selbst bleibt stabil.

### 4.2.3 Thumbnail-Worker (im bestehenden Background-Thread)

`gui/background.rs` um neuen Match-Arm erweitern:

```rust
BackgroundTask::LoadPhotoThumbnails { items } => {
    for (id, source) in items {
        let tx = result_tx.clone();
        let ctx = repaint_ctx.clone();
        pool.spawn(move || {
            match crate::thumbnail::load(&source, POOL_THUMB_MAX_EDGE_PX) {
                Ok((w, h, pixels)) => {
                    let _ = tx.send(BackgroundResult::PhotoThumbnailReady { id, width: w, height: h, pixels });
                    ctx.request_repaint();
                }
                Err(e) => tracing::warn!(%e, photo=%id, "thumb load failed"),
            }
        });
    }
}
```

Der Decoder lebt im GUI-Crate (`gui/thumbnail.rs`) — keine Kernfunktionalität
der Lib, wird nur vom Pool-Worker benötigt:

```rust
// gui/thumbnail.rs
use std::path::Path;
use anyhow::Result;

pub fn load(path: &Path, max_edge: u32) -> Result<(u32, u32, Vec<u8>)> {
    let img = image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .thumbnail(max_edge, max_edge); // nutzt EXIF-Thumb wenn vorhanden
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok((w, h, rgba.into_raw()))
}
```

Konstante `POOL_THUMB_MAX_EDGE_PX = 256` in `gui/app/widgets/photo_pool.rs`.

Tests (GUI-Crate): `thumbnail::decodes_jpeg`, `thumbnail::respects_max_edge`,
`thumbnail::missing_file_errors`. Fixtures aus `tests/` recyceln (kleines
JPEG bereits vorhanden).

### 4.2.4 Upload, Duplikat-Schutz und Stale-Drop

`FotobuchApp::drain_results` um `PhotoThumbnailReady` erweitern:

```rust
BackgroundResult::PhotoThumbnailReady { id, width, height, pixels } => {
    // Stale-Schutz: wenn id nicht mehr im Pool, stillschweigend verwerfen
    if !self.state.derived.photo_by_id.contains_key(&id) {
        self.state.photo_thumb_in_flight.remove(&id);
        continue;
    }
    let img = ColorImage::from_rgba_unmultiplied([width as _, height as _], &pixels);
    let tex = ctx.load_texture(format!("thumb_{id}"), img, TextureOptions::LINEAR);
    self.state.photo_thumbs.insert(id.clone(), tex);
    self.state.photo_thumb_in_flight.remove(&id);
}
```

### 4.2.5 Prefetch-Queue & Viewport-First-Dispatch

Pro Frame sammelt `photo_pool::draw` die IDs der **sichtbaren, noch ungeladenen
und nicht in-flight** Zeilen. Am Ende des Frames:

```rust
// gui/app/widgets/photo_pool.rs (am Ende von draw)
let visible_needed: Vec<String> = /* gesammelt in der Draw-Schleife */;
if !visible_needed.is_empty() {
    dispatch_thumb_load(state, cmds_out, visible_needed);
} else if !state.photo_thumb_prefetch.is_empty() {
    // Nur wenn nichts Sichtbares offen ist → fill in kleinen Chunks
    let chunk: Vec<String> = state.photo_thumb_prefetch
        .drain(..THUMB_FILL_CHUNK.min(state.photo_thumb_prefetch.len()))
        .collect();
    dispatch_thumb_load(state, cmds_out, chunk);
}
```

`dispatch_thumb_load`:
- filtert IDs, die schon in `photo_thumbs` oder `photo_thumb_in_flight` sind,
- schlägt alle verbleibenden in `photo_thumb_in_flight` ein,
- sendet **einen** `LoadPhotoThumbnails`-Task mit `(id, source_path)`-Paaren.

Die Prefetch-Queue wird einmal beim Start (nach `DerivedState::rebuild`) und
nach jedem `CommandDone` befüllt: alle IDs aus `derived.photo_by_id`, die weder
geladen noch in-flight sind. Reihenfolge irrelevant — Visible hat Vorrang.

Konstante `THUMB_FILL_CHUNK = 8` in `photo_pool.rs`.

### 4.2.6 Commits (Teil 1)

1. `feat(lib): thumbnail loader using image crate`
2. `feat(gui): photo thumbnail worker + prefetch queue`

Tests:
- `thumbnail::*` (GUI-Crate, s. o.).
- `pool::selection_*` (rein logisch).
- `photo_pool::dispatch_thumb_load_skips_cached_and_in_flight` (pure, mockt
  `photo_thumbs` + `photo_thumb_in_flight`).

---

## 4.2 Teil 2 — Pool-UI, Drag, Place

### 4.2.7 Layout

Linkes `SidePanel::left("photo_pool")`, Breite ≈ 220 px, resizable. Inhalt in
`egui::ScrollArea::vertical`. Gruppen aus `project_state.photos` iterieren,
sortiert nach `PhotoGroup.sort_key`. Pro Gruppe eine `CollapsingHeader`
(Default: offen) mit Titel `{group.name}` (bei Einzelgruppe ohne Namen: Ordner-
Basename). Pro `PhotoFile` eine Row.

### 4.2.8 Row

Eine `ui.horizontal`-Zeile pro Foto:

```
[24×24 thumb | filename (eliding)    | •]
```

- Thumb-Cell: `Image::from_texture(tex)` wenn vorhanden, sonst grauer
  Placeholder (`ui.painter().rect_filled(...)`). Wird in `visible_ids_this_frame`
  eingetragen für die Worker-Priorisierung (4.2.5).
- Filename: `file_name().to_string_lossy()`, Ellipse links wenn zu lang.
- Badge-Punkt (rechts), dreistufig (Breite fix, damit Rows nicht zittern):
  - unsichtbar: Foto nicht platziert (`placed_count == 0`),
  - grün: **genau einmal** platziert (`placed_count == 1`),
  - rot: **mehrfach** platziert (`placed_count > 1`, s. 4.2.14).
  Hover auf den Punkt öffnet einen Tooltip, der für jede Platzierung genau eine
  Zeile im Format `Page {p} Slot {s}` ausgibt (Reihenfolge: page asc, dann
  slot asc). Umsetzung via eigener `ui.interact`-Rect für den Punkt +
  `.on_hover_ui(|ui| …)` — der Lupen-Tooltip der Row wird dadurch **nicht**
  ausgelöst, weil der Punkt die Pointer-Interaction konsumiert.
- Die gesamte Row ist **eine** `ui.interact`-Area mit stabilem `Id`
  (`egui::Id::new(("pool_row", id))`), liefert `Response` für Click/Drag/Hover.

### 4.2.9 Hover-Lupe

Wenn `row_response.hovered()` und `photo_thumbs.get(id)` vorhanden:

```rust
row_response.on_hover_ui_at_pointer(|ui| {
    let size = vec2(tex.size_vec2().x, tex.size_vec2().y);
    ui.image((tex.id(), size));
});
```

`on_hover_ui_at_pointer` hat egui-nativen Delay (~300 ms). Kein eigener Timer.
Pop-up zeigt Thumb in nativer 256 px-Größe — reicht als „Lupe“. Wenn Thumb noch
nicht geladen: ID in `visible_ids_this_frame` eintragen, Tooltip zeigt Spinner
(`ui.spinner()`).

### 4.2.10 Click-Handling (Selection)

```rust
if row_response.clicked() {
    let order: Vec<String> = /* aus aktueller UI-Iteration aufgesammelt */;
    let mods = ui.input(|i| i.modifiers);
    if mods.shift { state.pool_selection.range_to(id, &order); }
    else if mods.ctrl || mods.command { state.pool_selection.toggle(id); }
    else { state.pool_selection = PoolSelection::single(id); }
}
```

`order` ist die Reihenfolge aller sichtbaren IDs in dieser Frame-Iteration —
wird außerhalb der Row-Schleife als `Vec<String>` aufgebaut und in
`range_to` übergeben. Das hält `PoolSelection` von UI-State frei und testbar.

### 4.2.11 Drag-Start (Pool → Page)

Rechte Maustaste auf Row:

```rust
if row_response.secondary_clicked() && matches!(state.pool_drag, Idle) {
    let ids = if state.pool_selection.contains(id) {
        state.pool_selection.ids().clone()
    } else {
        vec![id.clone()].into_iter().collect()
    };
    state.pool_drag = PoolDragState::Dragging { photo_ids: ids };
}
```

*Hinweis:* identische RMB-Semantik wie bei Slot-Drag im Central (s.
`input_handler::handle_drag_start`). Pool- und Slot-Drag schließen sich
gegenseitig aus, weil `pool_drag=Dragging` nur startet, wenn `drag=Idle`, und
umgekehrt.

### 4.2.12 Drop + Abbruch

In `input_handler::handle` ergänzen (vor `handle_drag_complete`):

```rust
fn handle_pool_drag_complete(state, ctx, cmds) -> bool {
    if !ctx.input(|i| i.pointer.secondary_released()) { return false; }
    let PoolDragState::Dragging { photo_ids } = std::mem::take(&mut state.pool_drag)
        else { return false; };
    if let Some(dst_page) = state.hovered_page {
        // Pool→Page-Drag liefert immer eine konkrete Zielseite.
        cmds.insert(PendingCommand::Place {
            photo_ids: photo_ids.into_iter().collect(),
            dst_page: Some(dst_page),
        });
        mark_page_dirty(state, dst_page);
    }
    true
}
```

Rückgabewert unterdrückt wie bei Slot-Drag den folgenden Click-Handler.

*Re-Place erlaubt:* Falls Fotos aus `photo_ids` bereits auf einer Seite liegen,
wird das nicht blockiert — sie werden zusätzlich platziert. Die daraus
resultierende Mehrfach-Platzierung wird in 4.2.14 optisch kenntlich gemacht und
kann vom User per Unplace selbst aufgelöst werden.

### 4.2.13 Blaue Page-Overlay während Pool-Drag

In `draw_page` nach Zeichnen der Slot-Overlays:

```rust
if matches!(state.pool_drag, PoolDragState::Dragging { .. })
    && state.hovered_page == Some(page_idx)
{
    ui.painter().rect_filled(page_rect, 0.0,
        Color32::from_rgba_unmultiplied(64, 128, 255, 48));
}
```

Kein per-Slot-Highlight — Pool-Drop ist seitenweit.

### 4.2.14 Duplikat-Indikatoren

Drei Stellen teilen sich dieselbe Datenquelle — einen neuen Eintrag in
`DerivedState`:

```rust
// gui/state/derived.rs
pub struct DerivedState {
    // … bestehende Felder …
    /// photo-id → Liste aller Slots, die das Foto belegen. Leere Liste =
    /// Foto liegt im Pool (nicht platziert). `rebuild` baut das in einem
    /// Durchlauf über `layout` auf.
    pub placed_locations: HashMap<String, Vec<(usize /*page*/, usize /*slot*/)>>,
}

impl DerivedState {
    pub fn placed_count(&self, id: &str) -> usize {
        self.placed_locations.get(id).map(|v| v.len()).unwrap_or(0)
    }
    pub fn duplicate_ids(&self) -> impl Iterator<Item = &String> {
        self.placed_locations.iter().filter(|(_,v)| v.len() > 1).map(|(k,_)| k)
    }
}
```

`photo_is_placed(id) = placed_count(id) > 0` — bestehende Callsites ziehen mit.

**Drei Indikatoren lesen daraus:**

1. **Pool-Row-Badge** (4.2.8): Farbe aus `placed_count`; Tooltip-Zeilen aus
   `placed_locations[id]`.
2. **Statusbar-Segment** in `widgets/statusbar.rs`, nur sichtbar wenn
   `derived.duplicate_ids().count() > 0`:

   ```rust
   let dup_count = state.derived.duplicate_ids().count();
   if dup_count > 0 {
       let resp = ui.label(format!("· {dup_count} Duplikate"));
       if resp.hovered() { state.highlight_duplicates = true; }
   }
   ```

   Neues GuiState-Feld `highlight_duplicates: bool` — wird pro Frame am
   Ende von `ui()` wieder auf `false` gesetzt (einfach zu resetten, kein
   komplexer Lifecycle).
3. **Central-Overlay**: wenn `state.highlight_duplicates` gesetzt, zeichnet
   `draw_page` über jeden Slot, dessen `photo_id` in
   `derived.duplicate_ids()` enthalten ist, ein rotes Overlay
   (`Color32::from_rgba_unmultiplied(220, 40, 40, 80)`). Lookup einmal pro
   Frame cachen (`HashSet<&str>`), nicht pro Slot neu iterieren.

Tests (pur):
- `derived::placed_locations_counts_across_pages`.
- `derived::duplicate_ids_lists_only_multiply_placed`.
- `statusbar::duplicate_segment_hidden_when_none`.

### 4.2.15 Toolbar-Button „Place“ + Hotkey `P`

Das Lib-Command `place` ohne `--into-page` verteilt Fotos automatisch
zeitstempel-basiert auf die Seiten. Der Button braucht daher **keine
Zielseite** — er ist immer aktivierbar, sobald die Pool-Selektion nicht leer
ist.

`PendingCommand::Place` wird entsprechend auf optionale Zielseite erweitert:

```rust
PendingCommand::Place { photo_ids: Vec<String>, dst_page: Option<usize> }
```

- Drag Pool → Seite (4.2.12) setzt `dst_page = Some(hovered_page)`.
- Button / Hotkey setzt `dst_page = None` → Lib verteilt automatisch.

In `toolbar.rs`:

```rust
let enabled = !state.pool_selection.is_empty();
if ui.add_enabled(enabled, egui::Button::new("Place")).clicked() {
    cmds.insert(PendingCommand::Place {
        photo_ids: state.pool_selection.ids().clone(),
        dst_page: state.hovered_page,   // Some → into page, None → auto
    });
}
```

In `input_handler::handle`:

```rust
fn handle_place_hotkey(state, ctx, cmds) {
    if !ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::P)) { return; }
    let ids = state.pool_selection.ids();
    if ids.is_empty() { return; }   // silent no-op
    cmds.insert(PendingCommand::Place {
        photo_ids: ids.clone(),
        dst_page: state.hovered_page, // same semantics as button
    });
}
```

Auf welche Seiten sich das Auto-Place auswirkt, entscheidet erst das
Lib-Command im Worker — daher markiert der Dispatcher in diesem Fall **alle**
`page_dirty`-Einträge (pauschal, analog zu `ConfigSet`), bevor er den Task
sendet.

### 4.2.16 Commits (Teil 2)

3. `feat(gui): photo pool panel with selection + hover preview`
4. `feat(gui): drag photos from pool to page`
5. `feat(gui): duplicate indicators in pool, statusbar, central overlay`
6. `feat(gui): place button and P hotkey (auto-distribute without target)`

Tests:
- `input_handler::place_hotkey_emits_place_with_hovered_page`
  (→ `dst_page = Some(_)`).
- `input_handler::place_hotkey_emits_place_without_target_when_no_hover`
  (→ `dst_page = None`).
- `input_handler::place_hotkey_no_op_when_selection_empty`.
- `input_handler::pool_drag_complete_emits_place_on_hovered_page`.
- `input_handler::pool_drag_complete_cancels_without_hovered_page`.

---

## 4.3 Config-Panel (floating Window)

### 4.3.1 Öffnen / Schließen

`Ctrl+,` als Toggle (egui-konvention analog zu VS Code). In
`input_handler::handle`:

```rust
if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Comma)) {
    state.config_panel.open = !state.config_panel.open;
}
```

Zusätzlich Button in Toolbar („⚙ Config“). Fenster schließt über das X der
`egui::Window`-Titelleiste; der `open: &mut bool`-Parameter kümmert sich.

### 4.3.2 State

```rust
pub struct ConfigPanelState {
    pub open: bool,
    /// Bearbeitungspuffer pro Pfad (dot-notation key → rohe String-Eingabe).
    /// Einträge verschwinden, sobald das Feld den Focus verliert und
    /// committed wurde — oder bei Escape.
    pub edit_buffers: HashMap<String, String>,
}
```

Kein eigener Snapshot der `BookConfig` — die aktuelle Wahrheit ist
`state.project_state.config`. Buffer speichert nur pending Edits.

### 4.3.3 Window-Rendering

`egui::Window::new("Config")`, `default_size([380, 520])`, `resizable(true)`.
Inhalt: `egui::ScrollArea::vertical` → rekursiver Walker über
`serde_yaml::to_value(&state.project_state.config.book)` (plus `.project`).

### 4.3.4 Rekursiver Widget-Walker

```rust
fn walk(ui, state, path: &str, value: &serde_yaml::Value) {
    match value {
        Mapping(m) => for (k, v) in m {
            let key = k.as_str().unwrap_or("?");
            let child_path = if path.is_empty() { key.to_string() }
                             else { format!("{path}.{key}") };
            if v.is_mapping() {
                CollapsingHeader::new(key).default_open(true)
                    .show(ui, |ui| walk(ui, state, &child_path, v));
            } else {
                ui.horizontal(|ui| {
                    ui.label(key);
                    leaf_widget(ui, state, &child_path, v);
                });
            }
        },
        Sequence(_) => ui.label("<list — read-only in Phase 4>"),
        _ => unreachable!("top-level is a mapping"),
    }
}
```

### 4.3.5 Leaf-Widget + commit-on-blur

```rust
fn leaf_widget(ui, state, key: &str, current: &serde_yaml::Value) {
    let current_str = match current {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        _ => return, // nested handled by walk()
    };
    let buf = state.config_panel.edit_buffers
        .entry(key.to_string()).or_insert_with(|| current_str.clone());
    let resp = ui.text_edit_singleline(buf);
    if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter) || !i.key_pressed(Key::Escape)) {
        if *buf != current_str {
            cmds.insert(PendingCommand::ConfigSet {
                key: key.to_string(), value: buf.clone(),
            });
        }
        state.config_panel.edit_buffers.remove(key);
    }
}
```

- `lost_focus` deckt Tab/Click-Away/Enter ab.
- Escape: Buffer verwerfen (egui setzt den Text beim nächsten Frame aus
  `current_str` zurück, weil Buffer entfernt wird).
- Bools zusätzlich als Checkbox anbieten (Typ-Check via `matches!(current,
  Value::Bool(_))`), commit sofort bei `changed()`.

### 4.3.6 Background-Dispatch

`PendingCommand::ConfigSet { key, value }` → `BackgroundTask::ConfigSet { key,
value, pixel_per_pt }`.

In `background.rs`-Worker:

```rust
BackgroundTask::ConfigSet { key, value, pixel_per_pt } => {
    let out = fotobuch::commands::config_set::execute(&state, &key, &value)?;
    if let Some(new_state) = out.changed_state {
        // komplettes Neu-Render: Page-Config (Größe/Ränder) kann betroffen sein
        rerender_all_pages(&new_state, pixel_per_pt, &result_tx);
        result_tx.send(CommandDone { new_state, ... }).ok();
    }
}
```

Da Config-Felder wie `book.page_size` oder `page.bleed` das Layout aller Seiten
ändern können, wird pauschal alles neu gerendert (kein selektives
Dirty-Tracking). Dauer akzeptabel — Config-Edits sind selten.

### 4.3.7 Commits

6. `feat(gui): recursive config widget walker`
7. `feat(gui): config panel with commit-on-blur + hotkey`

Tests:
- `config_panel::walk_flat_keys` (gegen `serde_yaml::Value` fixture).
- `config_panel::commit_only_on_change`.
- `config_panel::escape_discards_buffer`.

---

## 4.4 Akzeptanzkriterien

- [ ] Rechtes Nav-Panel zeigt Thumbnail + Nummer pro Seite, highlight bei
      Cursor auf Seite im Central.
- [ ] Klick auf Nav-Thumb scrollt Central zur Seite.
- [ ] RMB-Drag zwischen Nav-Thumbs tauscht Seiten (`PageSwap`).
- [ ] RMB-Drag vom Central-Slot auf Nav-Thumb verschiebt Slot cross-page.
- [ ] Linkes Pool-Panel listet alle `PhotoGroup`s mit Thumbnails (on-demand
      geladen, gecached).
- [ ] Hover auf Pool-Row zeigt 256 px-Lupe.
- [ ] Pool-Selection: Click / Ctrl+Click / Shift+Click funktionieren analog zur
      Slot-Selection.
- [ ] RMB-Drag Pool → Seite platziert ausgewählte Fotos auf diese Seite.
- [ ] Toolbar-Button „Place“ + `P` sind aktiv, sobald Selektion nicht leer ist;
      ohne gehoverte Seite verteilt die Lib zeitstempel-basiert auf alle Seiten,
      mit gehoverter Seite wird diese als Ziel genommen.
- [ ] Seite unter Pool-Drag wird blau getönt (flat, nicht per-slot).
- [ ] Badge am Pool-Eintrag: unsichtbar / grün / rot je nach
      `placed_count` 0/1/>1. Hover listet alle Platzierungen als
      `Page {p} Slot {s}`.
- [ ] Statusbar zeigt „· N Duplikate“ wenn mindestens ein Foto mehrfach
      platziert ist; Hover darauf überzieht alle betroffenen Slots im Central
      mit einem roten Overlay.
- [ ] Config-Fenster öffnet mit `Ctrl+,`, zeigt rekursiv alle BookConfig-Felder.
- [ ] Edit eines Feldes committed erst bei Blur/Enter; Escape verwirft.
- [ ] Config-Änderungen rendern alle Seiten neu.

## 4.5 Was NICHT in Phase 4

- Neue Seite erzeugen per Drag aufs Nav-Panel (Phase 5).
- Toast/Error-Popup bei fehlgeschlagenem Place (Phase 6).
- Add/Remove von Fotos aus `project_state.photos` (Phase 6).
- Persistenter Thumbnail-Cache auf Disk (Phase 7+, optional).
- Drag innerhalb des Pools (Reordering von `PhotoFile`s) — nicht vorgesehen.
- Listen-Editing in Config (Sequences bleiben read-only).
