# Phase 2: ProjectState + Slots + Selektion

**Ziel**: Der GuiState kennt das Projekt. Slots werden erkannt, gehighlighted
und selektiert. Weiterhin read-only — keine Commands, kein Drag & Drop.

**Voraussetzung**: Phase 1 abgeschlossen. `gui/state.rs`, `gui/app.rs`,
`gui/app/view.rs`, `gui/background.rs`, `gui/task.rs` existieren.

---

## 2.1 — ProjectState + DerivedState im GuiState

Bis jetzt hält `GuiState` nur `num_pages`. Phase 2 zieht den vollen
`ProjectState` hinein und legt den DerivedState daneben.

```rust
// gui/state.rs (erweitert)
pub struct GuiState {
    pub project_state: ProjectState,
    pub derived: DerivedState,
    pub page_textures: Vec<Option<TextureHandle>>,
    pub zoom: f32,
    pub base_pixel_per_pt: f32,
    pub selection: Selection,                  // 2.4
    pub hovered_slot: Option<(usize, usize)>,  // (page_idx, slot_idx)
    pub timings: Timings,
}
```

`GuiState::new(project_state)` leitet `num_pages` intern ab. `gui/main.rs`
lädt statt über den `StateManager` direkt per
`state_manager::load_project_state(&project_root, &project_name)` und reicht
den `ProjectState` rein.

### Neues Submodul `gui/state/derived.rs`

Kein `mod.rs`: `gui/state.rs` deklariert `mod derived;`. Felder wie in
`docs/design/gui/02-architektur.md` (`photo_by_id`, `placement_of_photo`,
`unplaced_photos`, `placed_per_group`, …). `rebuild(&ProjectState)` iteriert
einmal über `photos` und einmal über `layout[].photos` — kein inkrementelles
Patching.

Phase 2 ruft `rebuild` genau einmal beim Start im UI-Thread. Background-Offload
kommt in Phase 3 zusammen mit den Commands.

**Tests**:
- `rebuild_empty_state` → alle Maps leer.
- `rebuild_places_photos_into_placement_map` → Fixture mit platzierten und
  unplatzierten Fotos; prüft Count + einen konkreten `placement_of_photo`-Eintrag.
- `rebuild_group_lookup_resolves_each_photo_to_its_group`.

---

## 2.2 — Koordinaten mm → Screen (neues Modul `gui/app/geometry.rs`)

Slots liegen in `mm`, egui zeichnet in Pixeln. Der Übersetzer ist eine **pure
Funktion** in einem eigenen Modul, damit er ohne egui-App unittestbar bleibt.

```rust
pub fn slot_rect_on_screen(
    page_rect: egui::Rect,
    page_width_mm: f64,
    page_height_mm: f64,
    slot: &Slot,
) -> egui::Rect
```

Zoom ist bereits in `page_rect` eingerechnet (siehe `view::draw_page` aus
Phase 1). Keine Margen-Korrektur — Slot-Koordinaten sind absolut zur Seiten-Ecke.

**Tests**:
- `slot_at_origin_maps_to_page_min`.
- `slot_center_scales_linearly`.
- `full_page_slot_matches_page_rect`.

---

## 2.3 — Hit-Test + Hover

Ebenfalls in `gui/app/geometry.rs`:

```rust
pub fn hit_test_slot(
    pos: egui::Pos2,
    page_rect: egui::Rect,
    layout_page: &LayoutPage,
    page_width_mm: f64,
    page_height_mm: f64,
) -> Option<usize>
```

Iteration rückwärts durch die Slots, damit bei Überlappung der zuletzt
gezeichnete gewinnt.

### Overlay-Zeichnung in `view::draw_page`

`draw_page` gibt zusätzlich `Option<usize>` (hovered slot) zurück und zeichnet
Overlays mit `ui.painter()` in dasselbe `page_rect` wie das Image. Farbkonstanten
in `app/view.rs`:

- Hover: `Color32::from_rgba_unmultiplied(0, 120, 255, 38)` (~15 % blau).
- Selected: transparente Füllung + `Stroke::new(2.0, Color32::from_rgb(50, 200, 80))`.

`FotobuchApp::show_pages` sammelt die Rückgaben und schreibt `hovered_slot`
auf dem ersten Treffer.

**Tests** (in `app/geometry.rs`):
- `hit_test_outside_page_returns_none`.
- `hit_test_hits_last_overlapping_slot_first`.
- `hit_test_between_slots_returns_none`.

---

## 2.4 — Selektion (`gui/state/selection.rs`, neu)

```rust
pub enum Selection {
    None,
    OnPage {
        page: usize,
        slots: BTreeSet<usize>,  // sortiert → deterministische Tests
        anchor: usize,
    },
}

impl Selection {
    pub fn single(page: usize, slot: usize) -> Self;
    pub fn toggle(&mut self, page: usize, slot: usize);       // Ctrl+Klick
    pub fn range_to(&mut self, page: usize, slot: usize);     // Shift+Klick
    pub fn clear(&mut self);                                  // Escape / Leerklick
    pub fn select_all_on(&mut self, page: usize, slot_count: usize); // Ctrl+A
}
```

Selektion ist immer auf **eine Seite** beschränkt — `toggle`/`range_to` auf
einer anderen Seite werfen die alte Selektion weg und starten neu. `range_to`
ersetzt die gesamte Slot-Menge mit `min(anchor,slot)..=max(anchor,slot)`; Anker
bleibt stehen, damit mehrfaches Shift+Klick sauber "zieht".

### Input-Handling

In `FotobuchApp::handle_input` (neue Methode, wird aus `update` aufgerufen):

1. Klick → `hovered_slot` konsultieren → `single` / `toggle` / `range_to` /
   `clear`.
2. `Escape` → `clear`.
3. `Ctrl+A` → `select_all_on(current_page, …)`, wobei `current_page` = hovered,
   sonst die Seite der bestehenden Selektion, sonst no-op.

**Tests** (rein logisch, keine egui-Abhängigkeit):
- `single_replaces_previous`.
- `toggle_adds_then_removes`.
- `range_fills_between_anchor_and_new_forward_and_backward`.
- `click_on_other_page_clears_and_starts_new`.
- `select_all_picks_all_indices`.
- `clear_resets_to_none`.

---

## 2.5 — Layout-Migration: Toolbar + Statusbar

Phase 1 nutzt den eframe-Shortcut `fn ui(&mut self, ui, frame)`, der implizit
ein `CentralPanel` aufspannt. Für Top/Bottom-Panels **muss** auf
`fn update(&mut self, ctx, frame)` migriert werden.

```rust
impl eframe::App for FotobuchApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_results(ctx);
        self.handle_input(ctx);     // Zoom + Klick + Hotkeys
        self.request_repaint_if_loading(ctx);

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| toolbar::show(ui));
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            statusbar::show(ui, &self.state)
        });
        egui::CentralPanel::default().show(ctx, |ui| self.show_pages(ui));

        if self.state.timings.show {
            overlay::show_timings_overlay(&self.state.timings, ctx);
        }
    }
}
```

- **Toolbar** (`app/toolbar.rs`, neu): Stubs `[Build] [Release] [↩] [↪] [⚙]`
  als `ui.add_enabled(false, Button::new(…))`. Keine Hotkeys in Phase 2 — die
  werden in Phase 3 mit den Commands zusammen verdrahtet.
- **Statusbar** (`app/statusbar.rs`, neu): `Seite {hovered}/{total} · {photos}
  Fotos · {unplaced} unplatziert · Sel: {n} auf Seite {p}` oder `–`. Alle Werte
  kommen aus `GuiState` / `DerivedState`.

Der F2-Timings-Hotkey und Zoom wandern in `handle_input`.

---

## Akzeptanzkriterien

- [ ] `cargo build --features gui` ohne Warnings; `clippy --features gui -- -D warnings` sauber.
- [ ] `cargo fmt --check` und `cargo test` grün.
- [ ] GUI-Start: Toolbar oben (disabled Stubs), Statusbar unten, scrollbare Seitenansicht mittig.
- [ ] Hover über Slot → blaues Overlay + Statusbar-Update.
- [ ] Klick selektiert (grüne Umrandung); Klick auf freie Fläche räumt.
- [ ] Ctrl+Klick toggelt, Shift+Klick füllt Range vom Anker aus.
- [ ] Seitenwechsel per Klick verwirft die alte Selektion.
- [ ] Escape räumt; Ctrl+A wählt alle Slots der aktiven Seite.
- [ ] Statusbar-Zähler live aktuell.

## Was NICHT in Phase 2 gehört

- Kein Drag & Drop, keine Commands/Mutationen (Phase 3).
- Kein Rechtsklick-Kontextmenü (Phase 6).
- Kein Foto-Pool, keine Seiten-Nav (Phase 4).
- Kein Re-Render bei Zoom (Phase 3).
- Kein Manual-Mode (Phase 6).
- Keine Toasts — Phase 2 hat keine neuen Fehlerquellen.
