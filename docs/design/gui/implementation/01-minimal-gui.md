# Phase 1: Minimal Viable GUI

**Ziel**: Seiten anzeigen, scrollen, zoomen. Kein UI-Freeze.

**Voraussetzungen** (Phase 0):
- `fotobuch::output::typst::render_pages()` liefert `Vec<RenderedPage>` mit straight-alpha RGBA.
- `CommandOutput<T>` + `GuiState::apply` existieren (werden in Phase 1 aber noch nicht genutzt).

---

## 1.1 — Crate-Setup + Grundgerüst

### Cargo.toml

```toml
[[bin]]
name = "fotobuch-gui"
path = "gui/main.rs"
required-features = ["gui"]

[features]
gui = ["dep:eframe", "dep:egui"]

[dependencies]
eframe = { version = "…", optional = true, default-features = false, features = ["default_fonts", "glow"] }
egui   = { version = "…", optional = true }
```

Validierung: `cargo build` (ohne Feature) darf eframe/egui **nicht** ins `Cargo.lock` resolven — Regressionstest in CI.

### Dateiskelett

```
gui/
├── main.rs          Binary-Root; deklariert: mod app; mod state; mod background; mod task;
├── app.rs
├── state.rs
├── background.rs
└── task.rs
```

Flaches Layout — kein `gui/gui/`-Zwischenordner. Das Binary braucht keinen `gui::`-Namespace, `main.rs` deklariert die Submodule direkt. Kein `mod.rs`, jedes Submodul ist eine Datei im selben Ordner wie `main.rs`.

### Inhalte

**`gui/main.rs`**
- `fn main() -> eframe::Result<()>`
- Lädt `ProjectState` aus CWD (bestehende Lib-API aus `state_manager`/`cache`).
- Startet Worker aus `background::spawn(project_root, project_name)` → erhält `(task_tx, result_rx)`.
- Sendet initialen Task `BackgroundTask::RenderPages { pages: (0..n).collect(), pixel_per_pt: 1.5 }`.
- `eframe::run_native("fotobuch", NativeOptions::default(), Box::new(|cc| Ok(Box::new(FotobuchApp::new(cc, state, task_tx, result_rx)))))`.
- Fehler beim State-Laden: `eprintln!` + Exit-Code 1 (Projekt-Dialog kommt in späterer Phase).

**`gui/task.rs`** — minimaler Start, wird in Phase 3 erweitert:
```rust
pub enum BackgroundTask {
    RenderPages { pages: Vec<usize>, pixel_per_pt: f32 },
}

pub enum BackgroundResult {
    PageRendered(fotobuch::output::typst::RenderedPage),
    Error(String),
}
```

**`gui/background.rs`**
- `pub fn spawn(project_root: PathBuf, project_name: String) -> (Sender<BackgroundTask>, Receiver<BackgroundResult>)`
- Ein `std::thread::spawn`-Worker, `crossbeam::channel::unbounded` für beide Richtungen.
- Worker-Loop: `while let Ok(task) = task_rx.recv()` → match auf `RenderPages` → ruft `render_pages()` und sendet **pro Seite** ein `PageRendered` (nicht gesammelt — UI soll inkrementell updaten).
- Fehler im Render → `BackgroundResult::Error(…)`, Worker läuft weiter.

**`gui/app.rs`** (Phase-1-Skelett)
- `pub struct FotobuchApp { state: GuiState, task_tx, result_rx }`
- `impl eframe::App::update` nur als Stub (wird in 1.2/1.3 gefüllt).

**Unit-Test (Phase 1.1)**
- `background::tests`: Worker nimmt `RenderPages { pages: vec![0], … }` für ein Fixture-Projekt entgegen und liefert genau ein `PageRendered { page: 0, … }` in unter N Sekunden (Timeout via `recv_timeout`). Fixture-Projekt aus bestehender `tests/`-Infrastruktur wiederverwenden.

---

## 1.2 — Initiales Rendering

**`gui/state.rs`**
```rust
pub struct GuiState {
    pub page_textures: Vec<Option<egui::TextureHandle>>,
    pub zoom: f32,
    pub base_pixel_per_pt: f32, // konstant in Phase 1, z.B. 1.5
}
```

- `GuiState::new(num_pages: usize) -> Self` — `page_textures = vec![None; num_pages]`, `zoom = 1.0`.
- `pub fn apply_rendered(state: &mut GuiState, ctx: &egui::Context, rendered: RenderedPage)` — baut `ColorImage::from_rgba_unmultiplied([w, h], &pixels)` und `ctx.load_texture(format!("page_{}", rendered.page), image, TextureOptions::LINEAR)`, schreibt in `page_textures[rendered.page]`.
- **Pixel-Format-Garantie**: `render_pages` liefert straight-alpha → `from_rgba_unmultiplied` ist korrekt. In einem Kommentar am Call-Site dokumentieren, nicht im Kommentar weit weg.

**`FotobuchApp::update`** (in `app.rs`)
- Am Frame-Anfang: `while let Ok(msg) = self.result_rx.try_recv() { match msg { PageRendered(r) => state::apply_rendered(&mut self.state, ctx, r), Error(e) => tracing::error!(%e), } }`
- `if self.state.page_textures.iter().any(Option::is_none) { ctx.request_repaint(); }` — sonst bleibt der Frame stehen bis zur nächsten User-Eingabe.

**Unit-Test (Phase 1.2)**
- `state::tests::apply_rendered_sets_slot`: baut ein Mini-`RenderedPage` (2×2 px), ruft `apply_rendered` mit einem `egui::Context::default()`, prüft `page_textures[page].is_some()`. Reichweite bewusst klein — echte Texturinhalte zu vergleichen ist nicht der Sinn.

---

## 1.3 — Hauptansicht

In `FotobuchApp::update`, nach dem Result-Drain, innerhalb `egui::CentralPanel::default().show(ctx, |ui| …)`:

- `egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| { for i in 0..state.page_textures.len() { draw_page(ui, state, i); } })`
- `draw_page` (frei in `app.rs`):
  - `ui.label(format!("Seite {i}"))` — 0-basiert (CLAUDE.md-Konvention).
  - Zielgröße: `let size = vec2(base_w * state.zoom, base_h * state.zoom);` — `base_w/h` aus der Projekt-Config (ProjectState hält Book-Maße).
  - Textur da: `ui.add(egui::Image::from_texture(tex).fit_to_exact_size(size));`
  - Sonst: `let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover()); ui.painter().rect_filled(rect, 0.0, egui::Color32::from_gray(200));`

### Zoom

Vor dem Panel, im `update`:
```rust
let ctrl_scroll = ctx.input(|i| if i.modifiers.ctrl { i.raw_scroll_delta.y } else { 0.0 });
if ctrl_scroll != 0.0 {
    self.state.zoom = state::apply_zoom_delta(self.state.zoom, ctrl_scroll);
}
```

`state::apply_zoom_delta(z: f32, delta: f32) -> f32` — pure Funktion:
- `z * 1.1_f32.powf(delta.signum())`
- `.clamp(0.1, 5.0)`

**Kein Re-Render bei Zoom in Phase 1** — GPU-Skalierung der vorhandenen Textur reicht. Zoom-basiertes Re-Rendering mit höherem `pixel_per_pt` kommt in Phase 3 zusammen mit dem vollständigen Task-Enum.

**Unit-Test (Phase 1.3)**
- `state::tests::zoom_delta`: Zoom-in (delta > 0) vergrößert, Zoom-out (delta < 0) verkleinert, Obergrenze 5.0 hält, Untergrenze 0.1 hält, `delta == 0.0` ist no-op.

---

## Akzeptanzkriterien

- [ ] `cargo build` baut nur `fotobuch`. `eframe`/`egui` **nicht** im resultierenden Dep-Graph (`cargo tree` prüfen).
- [ ] `cargo build --features gui` baut `fotobuch` + `fotobuch-gui` ohne Warnings.
- [ ] `cargo clippy --features gui -- -D warnings` sauber.
- [ ] `cargo fmt --check` sauber.
- [ ] `cargo run --bin fotobuch-gui --features gui` in einem Projekt-Root: Fenster öffnet sich **sofort** mit grauen Platzhaltern, Texturen poppen asynchron ein, kein sichtbarer Freeze beim Start.
- [ ] Vertikales Scrollen flüssig, Ctrl+Scroll zoomt stufenweise.
- [ ] `cargo test` inkl. neuer Tests (`background`, `state::apply_rendered`, `state::apply_zoom_delta`) grün.

## Was NICHT in Phase 1 gehört

Zur Abgrenzung (Scope-Creep vermeiden):

- Keine Panels (Foto-Pool, Seiten-Nav) — kommen in Phase 4.
- Keine Commands/Mutations — keine `BackgroundTask::RunCommand` in `task.rs` in Phase 1.
- Kein `DerivedState`, keine Selektion, kein Drag & Drop.
- Keine Toolbar, keine Hotkeys (außer Ctrl+Scroll für Zoom).
- Keine Re-Render-Debounce-Logik für Zoom.
