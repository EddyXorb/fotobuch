# Phase 6: Manual Mode + Polish

**Ziel**: Pool-Add/Remove, Manual-Mode (Free-Position + Resize) inklusive
Solver-Skip, sowie die Polish-Themen Zoom-Debouncing,
Render-Cancellation, Off-Screen-Thumbnails, Drag-Ghosts, Smooth Scrolling,
Kontextmenüs.

**Voraussetzung**: Phase 5 abgeschlossen. Hotkeys komplett,
[+]-Platzhalter, Cross-Page Drag laufen.

---

## Status quo (Referenzpunkte im Code)

- `LayoutPage.mode: PageMode` existiert (`src/dto_models/layout/layout_page.rs:30-36`).
  YAML-Backward-Compat via `#[serde(default)]` + `skip_serializing_if = "is_auto"`
  erledigt.
- Lib: `execute_pos(project_root, page, slots, PosConfig)` verschiebt /
  skaliert Slots einer **Manual**-Seite
  (`src/commands/page/pos.rs:67-129`). `PosConfig { position:
  Option<PosMode>, scale: Option<f64> }`, `PosMode::Relative { dx_mm, dy_mm }`
  bzw. `Absolute { x_mm, y_mm }`. Manual-Check hart eingebaut —
  Auto-Pages liefern `ValidationError::PageNotManual`.
- Lib: `execute_mode(project_root, PagesExpr, PageMode)` toggelt den Modus
  (`src/commands/page/mode.rs:21-54`).
- Lib: `commands::add::add(...)` und `commands::remove::remove(...)`
  existieren komplett (`src/commands/add.rs:85-211`, `remove.rs:209-271`),
  werden von der GUI aber noch nicht aufgerufen.
- **Solver-Lücke**: der Kommentar in `incremental_build.rs:68` („skip
  manual pages") hält nicht — die Schleife ruft `rebuild_single_page`
  uneingeschränkt auf, und `rebuild_single` in `src/commands/rebuild.rs:116`
  wirft `anyhow::bail!` bei `PageMode::Manual`. Phase 6.1 zieht diesen Fix
  auf Lib-Ebene mit.
- `Viewport::zoom` wird in `handle_zoom` sofort gesetzt
  (`gui/app/input_handler.rs:49-69`). Kein Debouncing, kein
  Re-Render-Trigger.
- `draw_drag_ghosts.rs` ist vorhanden, aber kein Vollbild-Drag-Ghost
  sondern nur Skeleton. Der UX sieht einen halbtransparenten Thumbnail-Stack
  am Cursor vor.
- Toasts: `BackgroundResult::CommandFailed(String)` wird in `drain_results`
  nur geloggt (siehe Phase-3-TODO). Kein UI-Feedback.

---

## 6.0 — Pool Add/Remove

Reihenfolge absichtlich vor Manual-Mode — dieser Schritt braucht keine
Solver-Änderungen und entschärft `Ctrl+O` aus Phase 5.3.7 zum echten
Feature.

### 6.0.1 Add via Dialog (`Ctrl+O`, Toolbar-Button)

`InteractionState::add_dialog` wird erweitert:

```rust
// gui/state/add_dialog.rs (neu)
#[derive(Default)]
pub struct AddDialogState {
    pub open: bool,
    /// Gewählte Pfade (native FileDialog oder manuelles Drag&Drop).
    pub pending_paths: Vec<PathBuf>,
    pub recursive: bool,
    pub weight_buffer: String,   // roh, geparst bei Commit
    pub source_filter: String,   // Regex, leer = kein Filter
}
```

Widget `gui/app/widgets/add_dialog.rs`:

```rust
pub fn show(ctx: &egui::Context, state: &mut GuiState, cmds: &mut HashSet<PendingCommand>) {
    if !state.interaction.add_dialog.open { return; }
    let mut open = state.interaction.add_dialog.open;
    egui::Window::new("Fotos hinzufügen").open(&mut open)
        .default_size([420.0, 320.0])
        .show(ctx, |ui| {
            if ui.button("Ordner wählen …").clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    state.interaction.add_dialog.pending_paths.push(dir);
                }
            }
            ui.checkbox(&mut state.interaction.add_dialog.recursive, "Rekursiv");
            ui.horizontal(|ui| {
                ui.label("Gewicht:");
                ui.text_edit_singleline(&mut state.interaction.add_dialog.weight_buffer);
            });
            ui.horizontal(|ui| {
                ui.label("Pfad-Filter (Regex):");
                ui.text_edit_singleline(&mut state.interaction.add_dialog.source_filter);
            });
            for p in &state.interaction.add_dialog.pending_paths {
                ui.label(p.display().to_string());
            }
            ui.separator();
            let can_submit = !state.interaction.add_dialog.pending_paths.is_empty();
            if ui.add_enabled(can_submit, egui::Button::new("Hinzufügen")).clicked() {
                let weight = state.interaction.add_dialog.weight_buffer
                    .parse::<f64>().unwrap_or(1.0);
                cmds.insert(PendingCommand::AddPhotos {
                    paths: std::mem::take(&mut state.interaction.add_dialog.pending_paths),
                    recursive: state.interaction.add_dialog.recursive,
                    weight,
                    source_filter: state.interaction.add_dialog.source_filter.clone(),
                });
                state.interaction.add_dialog.open = false;
            }
        });
    state.interaction.add_dialog.open = open;
}
```

Abhängigkeit `rfd = "0.14"` als optionales Feature (`gui`) in `Cargo.toml`
aufnehmen.

Neu in `pending.rs` und `task.rs`:

```rust
PendingCommand::AddPhotos {
    paths: Vec<PathBuf>,
    recursive: bool,
    weight: f64,
    source_filter: String,   // leer = kein Filter
}

BackgroundTask::AddPhotos {
    paths: Vec<PathBuf>,
    recursive: bool,
    weight: f64,
    source_filter: String,
    pixel_per_pt: f32,
}
```

Worker (`gui/background/commands.rs`):

```rust
pub(super) fn run_add_photos(
    paths: Vec<PathBuf>, recursive: bool, weight: f64, source_filter: String,
    rctx: &mut super::RenderCtx<'_>,
) {
    let source_filters = if source_filter.is_empty() {
        vec![]
    } else {
        match regex::Regex::new(&source_filter) {
            Ok(re) => vec![re],
            Err(e) => {
                let _ = rctx.result_tx.send(BackgroundResult::CommandFailed(
                    format!("invalid filter regex: {e}")
                ));
                return;
            }
        }
    };
    let cfg = fotobuch::commands::add::AddConfig {
        paths, allow_duplicates: false, xmp_filters: vec![], source_filters,
        dry_run: false, update: false, recursive, weight,
    };
    match fotobuch::commands::add::add(rctx.project_root, &cfg) {
        Err(e) => { let _ = rctx.result_tx.send(BackgroundResult::CommandFailed(e.to_string())); }
        Ok(out) => {
            // Add modifiziert nur photos[], Layout bleibt gleich → keine dirty pages,
            // aber DerivedState muss neu gebaut werden (apply in drain_results).
            super::render::send_command_done(out.changed_state, vec![], rctx);
        }
    }
}
```

### 6.0.2 Add via OS-Drag (Dateien auf das Fenster)

eframe exposed `ctx.input(|i| i.raw.dropped_files)` — Liste
`Vec<egui::DroppedFile>` mit `path: Option<PathBuf>`. Im Input-Handler
neuer früher Schritt (vor `handle_drag_start`, weil Pointer-Drag weniger
Priorität hat):

```rust
fn handle_os_dropped_files(ctx: &egui::Context, cmds: &mut HashSet<PendingCommand>) {
    let paths: Vec<PathBuf> = ctx.input(|i| {
        i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect()
    });
    if paths.is_empty() { return; }
    cmds.insert(PendingCommand::AddPhotos {
        paths,
        recursive: true, // Ordner-Drop = rekursiv, Single-File-Drop verhält sich identisch
        weight: 1.0,
        source_filter: String::new(),
    });
}
```

Gilt UI-weit: der Drop landet im Pool — unabhängig davon, wo der Cursor
das File loslässt. Explizit nicht versucht: Drop in die Hauptansicht als
„place direkt auf Seite N", weil OS-Drag und Pool-Drag semantisch
überladen würden. (Wer direkt platzieren will: Add → dann Pool-Drag auf
Seite, siehe Phase 4.2.12.)

### 6.0.3 Remove via `Delete` im Pool

`handle_delete` aus Phase 5.3.1 hat schon drei Pfade in fester
Prioritätsreihenfolge:

1. Slot-Selektion → Unplace
2. Pool-Selektion (Phase-5-Hook ist leer)
3. Hovered Page ohne Selektion → DeletePage

Phase 6 füllt **nur den zweiten Pfad** — die Reihenfolge bleibt
identisch, damit eine bewusste Slot-Selektion nicht durch eine
versehentlich offene Pool-Selektion übertrumpft wird, und ein
Pool-Selection-Delete nicht versehentlich eine ganze hovered Page kippt:

```rust
fn handle_delete(
    data: &DataState,
    interaction: &mut InteractionState,
    ctx: &egui::Context,
    cmds: &mut HashSet<PendingCommand>,
) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)) { return; }

    // 1) Slot-Selektion gewinnt (Phase 5.3.1).
    if let SlotSelection::OnPage { page, slots, .. } = &interaction.selections.slots
        && !slots.is_empty()
    {
        cmds.insert(PendingCommand::Unplace {
            page: *page, slots: slots.iter().copied().collect(),
        });
        return;
    }

    // 2) Pool-Selektion (NEU in Phase 6).
    let pool_ids = interaction.selections.photos.ids();
    if !pool_ids.is_empty() {
        cmds.insert(PendingCommand::RemovePhotos { photo_ids: pool_ids });
        return;
    }

    // 3) Hovered Page → DeletePage (Phase 5.3.1).
    let target_page = interaction.hovered.as_ref().and_then(|h| match h {
        HoveredTarget::Page { page, slot: None } | HoveredTarget::NavPage(page) => Some(*page),
        _ => None,
    });
    if let Some(page) = target_page {
        if data.project.has_cover() && page == 0 { return; }
        cmds.insert(PendingCommand::DeletePage { page });
    }
}
```

Neu in `pending.rs` und `task.rs`:

```rust
PendingCommand::RemovePhotos { photo_ids: Vec<String> }
BackgroundTask::RemovePhotos { photo_ids: Vec<String>, pixel_per_pt: f32 }
```

Worker:

```rust
pub(super) fn run_remove_photos(
    photo_ids: Vec<String>,
    rctx: &mut super::RenderCtx<'_>,
) {
    // `patterns` in RemoveConfig erwartet Gruppenname ODER Regex auf source.
    // Wir übersetzen IDs → exakte `^id$`-Regexe. id == source hat im YAML
    // keine Garantie (id ist relpath, source kann anders sein), daher
    // brauchen wir den Photo-Index aus dem aktuellen State. Weil der Worker
    // den State neu lädt (StateManager::open), laden wir ihn einmal und
    // ziehen die source-Felder via DerivedState.rebuild — oder direkt aus
    // state.photos. Simpler: eine neue Lib-Funktion `remove_by_ids(ids)`
    // in src/commands/remove.rs, weil Regex-Indirektion hier fragil ist.
    match fotobuch::commands::remove::remove_by_ids(rctx.project_root, &photo_ids) {
        Err(e) => { let _ = rctx.result_tx.send(BackgroundResult::CommandFailed(e.to_string())); }
        Ok(out) => {
            let dirty = out.result.pages_affected;
            super::render::send_command_done(out.changed_state, dirty, rctx);
        }
    }
}
```

**Lib-Change** (minimal): `remove_by_ids(project_root: &Path, ids: &[String])
-> Result<CommandOutput<RemoveResult>>` als Thin-Wrapper um die bestehende
Remove-Pipeline — überspringt die Pattern-Match-Phase und setzt
`matched_ids = ids.iter().cloned().collect()` direkt. Implementiert in
`src/commands/remove.rs`, keine API-Änderung an `remove()` selbst.

`keep_files` bleibt `false` — Delete im Pool entfernt vollständig. Wer nur
unplacen will, selektiert im Central-Panel. Die UX-Regel („Pool-Fotos
Del → removed komplett") ist damit eingehalten.

### 6.0.4 Commits + Tests

1. `feat(lib): remove_by_ids helper for id-based removal`
2. `feat(gui): add photos dialog (Ctrl+O / toolbar)`
3. `feat(gui): OS file drop adds photos to pool`
4. `feat(gui): delete key removes selected pool photos`

Tests:
- `commands::remove::remove_by_ids_removes_exact_matches_only`.
- `input_handler::delete_prefers_pool_selection_over_slot_when_slot_empty`.
- `input_handler::delete_keeps_slot_path_when_both_selections_populated`
  (Slot gewinnt, weil Pool nur greift, wenn `SlotSelection::None`).
- `add_dialog::submit_consumes_pending_paths`.

---

## 6.1 — Manual Mode

### 6.1.1 Lib: Solver-Skip für Manual-Pages — Single Source of Truth

Aktuell hat die Lib **drei** Stellen, an denen ein „skip Manual" stehen
müsste (`incremental_build`, `rebuild_range`, `rebuild_all`) plus den
Book-Layout-Solver. Drei verstreute `if mode == Manual { continue; }`-
Klauseln sind exakt der Code-Smell, den der `incremental_build.rs:68`-
Kommentar bereits illustriert (er verspricht Skip, aber niemand setzt es
um). Phase 6 ersetzt das durch **einen einzigen Helper** in
`src/dto_models/layout.rs` (oder dem bestehenden Layout-Submodul):

```rust
impl ProjectState {
    /// Iteriert über Indizes der Pages, die der Solver anfassen darf.
    /// Manual-Pages werden ausgeschlossen — sie sind explizit fixiert.
    pub fn auto_page_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.layout.iter().enumerate()
            .filter(|(_, p)| p.mode == PageMode::Auto)
            .map(|(i, _)| i)
    }
}
```

**Drei Aufrufer migrieren** (alle bisherigen Schleifen über
`pages_needing_rebuild`/`range` rufen `auto_page_indices().filter(...)`
oder `pages.iter().filter(|p| state.layout[*p].mode == Auto)` — eine
Zeile pro Stelle, keine duplizierte Skip-Regel mehr):

| Datei | Heute | Nach 6.1.1 |
|---|---|---|
| `src/commands/build/incremental_build.rs:68` | Kommentar verspricht Skip, Code skippt nicht | filter via `auto_page_indices` |
| `src/commands/rebuild.rs::rebuild_range` | kein Skip | filter via `auto_page_indices` |
| `src/commands/rebuild.rs::rebuild_all` | kein Skip | filter via `auto_page_indices` |

`pages_rebuilt` im `BuildResult` enthält danach nur die wirklich neu
geplanten Indizes — automatisch, weil die Sammlung jetzt aus dem
gefilterten Iterator entsteht.

**Book-Layout-Solver** (`src/solver/book_layout_solver.rs`): Manual-Pages
werden als Fixed-Size-Blöcke an ihren Indizes belassen — der Solver
verteilt nur die Fotos der Auto-Pages neu. Umsetzung:

1. Vor dem Solver-Lauf einen Snapshot `Vec<(usize, LayoutPage)>` aller
   Manual-Pages anlegen (genau über `auto_page_indices` invertiert,
   damit dieselbe Wahrheit gilt).
2. Solver bekommt nur die Fotos der Auto-Pages.
3. Nach dem Lauf: Manual-Pages per `layout.splice` an ihren
   ursprünglichen Indizes wieder einsetzen.

Damit existiert die „Manual-Page wird nicht angefasst"-Regel **genau
einmal** in der Lib. Eigener Commit
`refactor(lib): single Manual-skip helper for solver and rebuilders`,
direkt vor dem GUI-Toggle-Commit.

### 6.1.2 Hotkey `A` → Modus toggeln

```rust
fn handle_mode_toggle(interaction: &InteractionState, ctx: &egui::Context, cmds: &mut HashSet<PendingCommand>) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::A)) { return; }
    let page = match &interaction.selections.slots {
        SlotSelection::OnPage { page, .. } => *page,
        SlotSelection::None => match &interaction.hovered {
            Some(HoveredTarget::Page { page, .. }) | Some(HoveredTarget::NavPage(page)) => *page,
            _ => return,
        },
    };
    cmds.insert(PendingCommand::TogglePageMode { page });
}
```

Neu in `pending.rs`:

```rust
PendingCommand::TogglePageMode { page: usize }
BackgroundTask::SetPageMode { page: usize, mode: fotobuch::dto_models::PageMode, pixel_per_pt: f32 }
```

Der `TogglePageMode`-Dispatcher liest den aktuellen Modus aus
`data.project.layout[page].mode`, berechnet `mode.toggle()` und sendet
`SetPageMode`. Toggle-Logik liegt damit im UI-Thread (kein Read-Modify-Write
über mehrere Worker-Roundtrips).

Worker:

```rust
pub(super) fn run_set_page_mode(page: usize, mode: PageMode, rctx: &mut super::RenderCtx<'_>) {
    use fotobuch::commands::page::{execute_mode, PagesExpr};
    match execute_mode(rctx.project_root, PagesExpr::single(page as u32), mode) {
        Err(e) => { let _ = rctx.result_tx.send(BackgroundResult::CommandFailed(e.to_string())); }
        Ok(out) => {
            // Manual→Auto: Seite wird beim nächsten Build neu geplant.
            //   run_page_command ruft build() → neue Slots kommen als dirty zurück.
            // Auto→Manual: Slots bleiben wie sie sind, nur `mode` flippt.
            //   Kein Build nötig, aber Derived muss neu gebaut werden.
            if mode == PageMode::Auto {
                super::commands::finish_with_build(out, rctx);
            } else {
                super::render::send_command_done(out.changed_state, vec![], rctx);
            }
        }
    }
}
```

### 6.1.3 `[A|M]`-Toggle-Button pro Seite

In `gui/app/widgets/central_panel/draw_page.rs`: nach dem eigentlichen
Page-Image ein kleiner Badge-Button rechts neben der Seite (innerhalb der
gleichen `ui.horizontal`-Zeile, nicht im Page-Overlay — würde Drop-Targets
kapern):

```rust
let label = match data.project.layout[page_idx].mode {
    PageMode::Auto   => "[A]",
    PageMode::Manual => "[M]",
};
if ui.small_button(label).clicked() {
    cmds.insert(PendingCommand::TogglePageMode { page: page_idx });
}
```

Analoges Badge in `page_nav::draw` (rechtes Panel) — aber dort **nicht
klickbar**, reiner Indikator. Klick im Nav ist bereits für Scroll-To
belegt.

### 6.1.4 Manual-Drag: Slot auf freie Fläche

In `draw_page` wird für jeden Slot einer **Manual**-Seite ein zusätzlicher
`egui::Sense::drag()`-Hotspot in der Slot-Rect gelegt, der primäre
Maustaste akzeptiert (LMB für Manual-Drag, damit sich der Semantik-Split
zu RMB-Slot-Drag = Swap/Move nicht beißt):

```rust
// Nur für Manual-Pages:
let slot_rect = geometry::slot_rect_on_screen(page_rect, w_mm, h_mm, slot);
let id = egui::Id::new(("manual_slot", page_idx, slot_idx));
let resp = ui.interact(slot_rect, id, egui::Sense::click_and_drag());
if resp.drag_started_by(egui::PointerButton::Primary) {
    interaction.manual_drag = ManualDrag::Move {
        page: page_idx, slot: slot_idx,
        pointer_origin: resp.hover_pos().unwrap_or(slot_rect.center()),
        slot_origin_mm: (slot.x_mm, slot.y_mm),
    };
}
```

Neuer State in `gui/state/drag.rs`:

```rust
#[derive(Default, Debug, Clone)]
pub enum ManualDrag {
    #[default]
    Idle,
    Move {
        page: usize, slot: usize,
        pointer_origin: egui::Pos2,
        slot_origin_mm: (f64, f64),
    },
    Resize {
        page: usize, slot: usize,
        corner: Corner, // NW|NE|SW|SE
        pointer_origin: egui::Pos2,
        slot_origin_mm: (f64, f64, f64, f64), // x, y, w, h
    },
}

#[derive(Debug, Clone, Copy)]
pub enum Corner { NW, NE, SW, SE }
```

`InteractionState::manual_drag: ManualDrag` (neues Feld).

**Drag-Fortschritt** (bei `resp.dragged()` oder global pro Frame):

1. Aktuelle Cursor-Position `pointer_now`.
2. Delta in Pixel: `pointer_now - pointer_origin`.
3. Delta in mm: `delta_px / pixel_per_mm`, mit
   `pixel_per_mm = page_rect.width() / page_width_mm`.
4. Neue Slot-Position = `(slot_origin_mm.0 + dx_mm, slot_origin_mm.1 + dy_mm)`.

Die neue Position wird **optimistisch** als lokales Overlay gezeichnet —
der Slot selbst im `LayoutPage` bleibt unverändert, bis der Drag loslässt.
Das spart Roundtrip-Latenz und macht den Drag ruckelfrei.

**Drag-Release** (`resp.drag_stopped_by(Primary)` bzw.
`ctx.input(|i| i.pointer.primary_released())`):

```rust
let ManualDrag::Move { page, slot, slot_origin_mm, pointer_origin } =
    std::mem::take(&mut interaction.manual_drag) else { return; };
let (dx_mm, dy_mm) = compute_delta_mm(pointer_origin, pointer_now, pixel_per_mm);
cmds.insert(PendingCommand::PagePos {
    page, slot,
    mode: PagePosMode::Relative { dx_mm, dy_mm },
    scale: None,
});
```

Auto-Pages ignorieren den Drag vollständig (`ui.interact` wird für Slots
auf Auto-Seiten nicht registriert). Kein visuelles Feedback nötig — der
bestehende RMB-Swap/Move bleibt die einzige Drag-Semantik dort.

### 6.1.5 Manual-Resize: Ecken-Drag

Für Manual-Slots zusätzlich vier `ui.interact`-Hotspots in 8×8-Pixel-Rects
an den Ecken:

```rust
fn corner_rect(slot_rect: egui::Rect, corner: Corner) -> egui::Rect {
    const SZ: f32 = 8.0;
    let c = match corner {
        Corner::NW => slot_rect.left_top(),
        Corner::NE => slot_rect.right_top(),
        Corner::SW => slot_rect.left_bottom(),
        Corner::SE => slot_rect.right_bottom(),
    };
    egui::Rect::from_center_size(c, egui::vec2(SZ, SZ))
}
```

Spezifikation als **pure Funktion** in `gui/app/widgets/central_panel/manual_resize.rs`:

```rust
pub fn compute(
    origin: (f64, f64, f64, f64), // x, y, w, h in mm
    corner: Corner,
    delta_px: egui::Vec2,
    pixel_per_mm: f64,
) -> (f64, f64, f64, f64) // new x, y, w, h in mm
```

Invarianten (unittests, jeweils eine Assertion):
- `compute_keeps_aspect_ratio`: `new_w / new_h ≈ origin_w / origin_h` (eps).
- `compute_se_corner_keeps_origin_xy`: SE bewegt nur Größe.
- `compute_nw_corner_shifts_origin_xy_by_size_delta`: NW behält gegenüber-
  liegende Ecke (`origin_x + origin_w`, `origin_y + origin_h`) fix.
- `compute_zero_delta_is_identity`.

Implementierung: Distanz Cursor → Gegenecke vor/nach Drag, Scale =
`new_diag / origin_diag`, neue Größe = `origin_size * scale`, neue Origin
so, dass die Gegenecke fix bleibt. Welche Ecke gegenüberliegt, bestimmt
`Corner` (NW ↔ SE, NE ↔ SW). 8 Zeilen Code, 4 Unit-Tests, kein Prosa-
Pseudocode nötig.

**Release** emittiert:

```rust
cmds.insert(PendingCommand::PagePos {
    page, slot,
    mode: PagePosMode::Absolute { x_mm: new_x, y_mm: new_y },
    scale: Some(scale_factor),
});
```

`PosConfig` akzeptiert beides in einem Call (siehe `src/commands/page/pos.rs:31-36`).

Neu in `pending.rs` und `task.rs`:

```rust
pub enum PagePosMode {
    Relative { dx_mm: f64, dy_mm: f64 },
    Absolute { x_mm: f64, y_mm: f64 },
}

PendingCommand::PagePos { page: usize, slot: usize, mode: PagePosMode, scale: Option<f64> }
BackgroundTask::PagePos  { page: usize, slot: usize, mode: PagePosMode, scale: Option<f64>, pixel_per_pt: f32 }
```

Worker:

```rust
pub(super) fn run_page_pos(page: usize, slot: usize, mode: PagePosMode, scale: Option<f64>, rctx: &mut super::RenderCtx<'_>) {
    use fotobuch::commands::page::{execute_pos, PosConfig, PosMode, SlotExpr};
    let cfg = PosConfig {
        position: Some(match mode {
            PagePosMode::Relative { dx_mm, dy_mm } => PosMode::Relative { dx_mm, dy_mm },
            PagePosMode::Absolute { x_mm, y_mm }   => PosMode::Absolute { x_mm, y_mm },
        }),
        scale,
    };
    match execute_pos(rctx.project_root, page as u32, SlotExpr::single(slot as u32), &cfg) {
        Err(e) => { let _ = rctx.result_tx.send(BackgroundResult::CommandFailed(e.to_string())); }
        Ok(out) => super::render::send_command_done(out.changed_state, vec![page], rctx),
    }
}
```

Kein `build()` nötig — `execute_pos` schreibt Slots direkt, Typst-Render
reagiert nur auf die geänderten Slot-Koordinaten.

### 6.1.6 Commits + Tests

1. `fix(lib): skip manual pages in rebuild/incremental build`
2. `feat(lib): book layout solver treats manual pages as fixed blocks`
3. `feat(gui): A hotkey + [A|M] toggle button per page`
4. `feat(gui): manual slot free-positioning via primary-drag`
5. `feat(gui): manual slot resize via corner drag`

Tests:
- `manual_resize::compute_keeps_ratio_for_se_corner`.
- `manual_resize::compute_shifts_origin_for_nw_corner`.
- `input_handler::a_hotkey_emits_toggle_page_mode_when_hovered`.
- `input_handler::manual_drag_release_emits_page_pos_relative`.
- Lib: `incremental_build_skips_manual_page` (Fixture mit einer Manual-Page,
  erwartet unverändertes `slots[]` nach Build).
- Lib: `book_layout_solver_preserves_manual_pages`.

---

## 6.2 — Polish

Sechs kleine, unabhängige Features. Reihenfolge egal, ein Commit pro
Feature. Wenn ein Feature neuen State braucht, wird der unmittelbar vor
dem Feature-Commit in `state/` angelegt.

### 6.2.1 Zoom-Debouncing

`Viewport::zoom` schreibt in `handle_zoom` (input_handler.rs:49-69) sofort.
Das genügt für die GPU-Skalierung der bestehenden Textur, aber der
eigentliche Re-Render mit höherem `pixel_per_pt` muss **erst nach Ruhe**
erfolgen, damit die Pipeline nicht bei jedem Scroll-Tick anläuft.

State:

```rust
// gui/state/viewport.rs
pub struct Viewport {
    // … bestehende Felder …
    /// Timestamp des letzten Zoom-Delta. `None` = stabil.
    pub zoom_last_change: Option<std::time::Instant>,
    /// Letzter `pixel_per_pt`-Wert, zu dem gerendert wurde.
    pub zoom_last_rendered_ppp: f32,
}
```

In `FotobuchApp::logic` (bzw. `update`-Äquivalent) nach `handle_input`:

```rust
const ZOOM_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(200);

if let Some(t) = self.state.interaction.viewport.zoom_last_change
    && t.elapsed() >= ZOOM_DEBOUNCE
{
    let ppp = self.state.interaction.viewport.zoom * self.state.data.pages.base_pixel_per_pt;
    let last = self.state.interaction.viewport.zoom_last_rendered_ppp;
    if (ppp - last).abs() / last > 0.05 {   // mehr als 5% Unterschied → Re-Render lohnt
        let pages: Vec<usize> = (0..self.state.data.pages.textures.len()).collect();
        let _ = self.task_tx.send(BackgroundTask::RenderPages { pages, pixel_per_pt: ppp });
        self.state.interaction.viewport.zoom_last_rendered_ppp = ppp;
    }
    self.state.interaction.viewport.zoom_last_change = None;
}
```

`handle_zoom` schreibt zusätzlich:

```rust
interaction.viewport.zoom_last_change = Some(std::time::Instant::now());
```

`ctx.request_repaint_after(ZOOM_DEBOUNCE)` in `handle_zoom`, damit der
Debounce-Tick auch ohne weitere Input-Events feuert.

Tests:
- `viewport::zoom_debounce_emits_after_quiet_period` (simuliert Instant
  mit `mock_instant` oder extrahiert die Entscheidung als pure Funktion
  `should_rerender(last_change, now, last_ppp, new_ppp) -> bool`).

### 6.2.2 Render-Cancellation

Das existierende `BackgroundResult::PageRendered`-Handling schluckt jedes
Rendering, auch wenn die Seite zwischendurch schon wieder neu angefragt
wurde. Bei schnellen Zoom-Sprüngen oder konsekutiven Commands stapeln sich
veraltete Pixelbuffer im Channel und überschreiben die frische Textur.

Lösung analog zu `docs/design/gui/02-architektur.md` § „Epoch-basierte
Invalidierung":

1. `PageCache` bekommt `render_epochs: Vec<u64>` (index-coupled mit
   `textures`).
2. `BackgroundTask::RenderPages` und `PageRendered` tragen `epoch: u64`.
3. Vor jedem `RenderPages`-Send inkrementiert die UI pro betroffener Seite
   `render_epochs[p]`.
4. `drain_results` verwirft `PageRendered { epoch, page }` still, wenn
   `epoch < render_epochs[page.page]`.

Task + Result:

```rust
BackgroundTask::RenderPages { pages: Vec<(usize, u64 /* epoch */)>, pixel_per_pt: f32 }
BackgroundResult::PageRendered {
    page: RenderedPage, thumb: RenderedPage,
    epoch: u64,
    rasterize_duration: Duration,
    compile_duration: Duration,
}
```

**Break-Change**: alle bestehenden `RenderPages`-Callsites in
`background/commands.rs` (nach `execute_move`, `place`, `config_set`, etc.)
müssen den Epoch-Wert mitgeben. Einfachstes Muster: Der UI-Thread baut
`Vec<(usize, u64)>` indem er nach jedem dirty-pages-bumping die neuen
Epochen sammelt und an den Task hängt — der Worker leitet sie nur durch.

Commit: `feat(gui): epoch-based render cancellation`.

Tests:
- `page_cache::bump_epochs_increments_only_listed_pages`.
- `app::stale_page_rendered_is_dropped`.

### 6.2.3 Thumbnails statt Full-Res für Off-Screen-Seiten

Im UI-Thread pro Frame in `draw_page` entscheiden, ob die Seite in der
aktuellen Viewport-Region liegt:

```rust
let viewport_top    = interaction.viewport.scroll.viewport_top;
let viewport_bottom = viewport_top + ui.available_height();
let visible = page_rect.max.y > viewport_top && page_rect.min.y < viewport_bottom;

if visible {
    if let Some(tex) = &data.pages.textures[page_idx] {
        ui.add(egui::Image::from_texture(tex).fit_to_exact_size(size));
    }
} else if let Some(thumb) = &data.pages.thumb_textures[page_idx] {
    ui.add(egui::Image::from_texture(thumb).fit_to_exact_size(size));
}
```

Full-Res-Texturen werden für nicht sichtbare Seiten **nicht** entladen —
nur nicht gezeichnet. Das ist billiger als Re-Upload beim nächsten Scroll
und kostet nur GPU-Memory.

**Dispatch-Regel**: `RenderPages`-Tasks anfragen nur für Seiten, die im
Viewport sichtbar sind ODER dirty sind. Off-Screen-Dirty-Pages warten, bis
sie in den Viewport scrollen. Umsetzung: neue `visible_pages()`-Helper aus
`scroll_offset + viewport_height` + `page_rect_cache` (Phase 3 bereits
vorgesehen) — rein UI-Thread-seitige Rechnung.

Commit: `feat(gui): use thumbs for off-screen pages, render only visible`.

Tests:
- `viewport::visible_pages_returns_intersecting_indices`.
- `app::off_screen_dirty_page_does_not_emit_render_task`.

### 6.2.4 Drag-Ghosts

Das bestehende `draw_drag_ghosts.rs` wird erweitert: Während
`ActiveDrag::Dragging(src)` zeichnet die UI auf `ctx.layer_painter(top)`
ein halbtransparentes Thumbnail des Drag-Inhalts am Cursor.

| Quelle | Ghost |
|---|---|
| `DragSource::Slot` | Das Slot-Rect mit **Alpha-gedimmtem Full-Page-Ausschnitt** (aus `page_textures`, geclippt auf die Slot-Koordinaten). Bei Multi-Selektion: pro zusätzlichem Slot leicht verschoben („Stack") + Badge `×N` rechts oben. |
| `DragSource::NavPage` | Das Page-Thumb gedimmt. |
| `DragSource::Pool { photo_ids }` | Das Pool-Thumb des **ersten** Fotos gedimmt, bei `>1 id` Badge `×N`. |

Umsetzung in einer pure Funktion
`ghost::draw(ctx, source, cursor_pos, data)`, aufgerufen am Ende von
`FotobuchApp::ui`. Kein Interaktions-Kapern — Ghost liegt im Overlay-Layer
(`egui::LayerId::new(egui::Order::Tooltip, Id::new("drag_ghost"))`) und
bekommt `Sense::hover()` nicht zugewiesen.

Commit: `feat(gui): drag ghosts for slot/nav/pool drags`.

Tests (nur Geometrie):
- `ghost::multi_slot_stack_offset_is_4_px_per_item`.

### 6.2.5 Smooth Scrolling

egui hat natives Scroll-Easing nicht an, aber zwei kleine Schrauben
reichen:

1. `ScrollArea::auto_shrink([false; 2])` ist gesetzt — OK.
2. `pending_scroll_y` in `Viewport::scroll` darf nicht hart gesetzt
   werden, sondern als Ease-Target; pro Frame wird der aktuelle
   `scroll_y` gegen das Ziel um einen Bruchteil `t = 0.25` interpoliert:

```rust
// in FotobuchApp::logic, nach handle_input:
if let Some(target_y) = self.state.interaction.viewport.scroll.ease_target {
    let current = self.state.interaction.viewport.scroll.scroll_y;
    let next = current + (target_y - current) * 0.25;
    self.state.interaction.viewport.scroll.pending_scroll_y = Some(next);
    if (target_y - next).abs() < 1.0 {
        self.state.interaction.viewport.scroll.ease_target = None;
    } else {
        ctx.request_repaint();
    }
}
```

Scroll-to-Page-Logik aus Phase 4.1.4 und die Home/End/Ctrl+G-Handler aus
Phase 5 setzen `ease_target` statt direkt `scroll_to_page` zu triggern.
Die bisherige Scroll-To-Rect-Mechanik (`ui.scroll_to_rect`) bleibt als
harter Sprung für `scroll_to_page` erhalten — Ease ist separat auf
expliziten Eingaben (Pagenum-Hotkeys, Nav-Klick).

Commit: `feat(gui): ease scroll transitions on page jumps`.

### 6.2.6 Kontextmenü (Rechtsklick)

Bislang bindet RMB den Slot/Nav/Pool-Drag. Für ein Kontextmenü braucht es
eine Abgrenzung: **RMB-Klick mit sofortigem Release** (Dauer < 150 ms,
Cursor-Bewegung < 4 px) öffnet das Menü, sonst ist es ein Drag-Start.

Umsetzung:

1. `handle_drag_start` (input_handler.rs:87) merkt sich den
   RMB-Press-Zeitpunkt in `interaction.drag.press_instant: Option<Instant>`
   — Drag wird aber nicht sofort aktiv, sondern erst, wenn entweder Maus
   >4 px bewegt oder Zeit > 150 ms überschritten.
2. `handle_drag_complete` prüft vor der bestehenden Source-Logik: wenn der
   Press-Zeitpunkt < 150 ms her ist und die Maus nicht bewegt hat,
   öffne stattdessen `interaction.context_menu = Some(ContextMenu::for(hovered))`.

`ContextMenu` als eigener State:

```rust
pub enum ContextMenu {
    Slot { page: usize, slot: usize, screen_pos: egui::Pos2 },
    Page { page: usize, screen_pos: egui::Pos2 },
    NavPage { page: usize, screen_pos: egui::Pos2 },
    PoolItem { id: String, screen_pos: egui::Pos2 },
    PoolGroup { group: String, screen_pos: egui::Pos2 },
}
```

Widget `gui/app/widgets/context_menu.rs` öffnet
`egui::Area::new("ctx_menu").fixed_pos(pos).show(ctx, |ui| …)` mit
Einträgen je nach Variante:

| Kontext | Einträge |
|---|---|
| Slot | Unplace · Rebuild · Weight … · Info |
| Page | Rebuild · Set Mode (Auto/Manual) · Info |
| NavPage | Set Mode · Rebuild · Delete Page (wenn leer) |
| PoolItem | Remove |
| PoolGroup | Remove Group · Place All From Group |

Jeder Eintrag emittiert einen bestehenden `PendingCommand`.
Weight-Eintrag öffnet einen Sub-DragValue (siehe `page weight` Command,
`src/commands/page/weight.rs` — existierende Lib).

Außerhalb des Menüs geklickt → schließen (`if !area.contains_pointer()`
und LMB-Press ⇒ `interaction.context_menu = None`).

Commit: `feat(gui): right-click context menus for slot/page/pool`.

Tests:
- `input_handler::quick_rmb_tap_opens_context_menu`.
- `input_handler::rmb_hold_or_drag_does_not_open_menu`.

### 6.2.7 Toast-System (nur Error)

Jedes `BackgroundResult::CommandFailed(msg)` landet bislang nur im Log.
Phase 6 zeigt **ausschließlich Error-Toasts** — `Info`/`Warning` sind
YAGNI (kein Caller existiert), kommen erst, wenn ein konkreter Bedarf
auftaucht.

```rust
// gui/state/toasts.rs
pub struct ErrorToast { pub message: String, pub shown_since: Instant }

pub struct ToastQueue { pub items: VecDeque<ErrorToast> }
impl ToastQueue {
    pub fn push(&mut self, msg: impl Into<String>) { /* TTL=6s gc inline */ }
}
```

`DataState::toasts: ToastQueue` (neues Feld). `drain_results` bei
`CommandFailed(msg)` → `data.toasts.push(msg)`. Widget
`gui/app/widgets/toasts.rs`: `egui::Area` bottom-right, rote Icon-Zeile
pro Toast, Auto-Fade nach 6 s.

Commit: `feat(gui): error toasts for command failures`.

---

## 6.3 Akzeptanzkriterien

- [ ] `Ctrl+O` öffnet Add-Dialog; Ordner wählen + „Hinzufügen" ruft
      `commands::add::add` im Worker auf, Pool wird danach aktualisiert.
- [ ] OS-Drag von Dateien/Ordnern auf das Fenster fügt Fotos zum Pool
      hinzu (rekursiv).
- [ ] `Delete` mit Pool-Selektion entfernt die Fotos komplett; mit
      Slot-Selektion unplaced.
- [ ] `[A|M]`-Button wechselt Seitenmodus. `A`-Hotkey auf hovered Seite
      gleichwertig.
- [ ] Build/Rebuild auf Projekten mit Manual-Pages läuft durch (skip statt
      Error). Manual-Page-Slots bleiben unverändert.
- [ ] LMB-Drag innerhalb eines Manual-Slots verschiebt diesen; Release
      commitet via `page pos`.
- [ ] LMB-Drag an einer Slot-Ecke (Manual) skaliert unter Beibehaltung des
      Seitenverhältnisses.
- [ ] Zoom ruckelfrei: Ctrl+Scroll skaliert sofort (GPU), Re-Render erst
      ~200 ms nach letzter Änderung, nur wenn >5 % ppp-Änderung.
- [ ] Nach mehreren Zoom-Tasks bleibt nur der jüngste Render-Output
      sichtbar, keine Flackerer (Epoch-Invalidierung).
- [ ] Off-Screen-Seiten zeigen das Nav-Thumb statt Full-Res.
- [ ] Drag-Ghost am Cursor während Slot/Nav/Pool-Drag, Multi-Selection
      zeigt `×N`-Badge.
- [ ] Scroll-to-Page animiert weich.
- [ ] Kurzer Rechtsklick öffnet Kontextmenü (Slot/Page/NavPage/Pool-Item/
      Pool-Group), Halten/Bewegen startet stattdessen Drag.
- [ ] Fehlgeschlagene Commands (z. B. Rebuild auf Manual-Page) erscheinen
      als roter Toast, der nach ~6 s verschwindet.
- [ ] `cargo build --features gui`, `cargo clippy --features gui --
      -D warnings`, `cargo fmt --check`, `cargo test` alle grün.

## 6.3.1 Code-Smell-Review (Self-Audit)

**1. Manual-Skip-Logik ist EIN Helper** (6.1.1).
`ProjectState::auto_page_indices()` ist die einzige Stelle, an der „nicht
Manual" definiert wird. Drei bisherige Aufrufer-Stellen (`incremental_build`,
`rebuild_range`, `rebuild_all`) und der Book-Layout-Solver-Snapshot
ziehen aus derselben Quelle.

**2. `remove_by_ids` als Thin-Wrapper, nicht als Regex-Roundtrip** (6.0.3).
Die GUI hat IDs zur Hand — den Umweg über `^id$`-Regex (mit
`source` ≠ `id`-Falle) zu nehmen wäre ein Code-Smell. Stattdessen ein
schmaler Lib-Helper, der die existierende `remove`-Pipeline nach der
Match-Phase anstößt.

**3. `ToastKind::Info` ist YAGNI** (6.2.7).
Im Phase-6-Plan gibt es keinen Caller für `Info`/`Warning`. Toast-Typ
wird auf `ErrorToast` reduziert. Erweiterung erst, wenn ein zweiter
Toast-Pfad aufkommt.

**4. ManualDrag und ActiveDrag bleiben getrennt — bewusst**.
Der erste Reflex ist „beide Drag-States in ein Enum mergen". Aber RMB-
Drag (Swap/Move/Cross-Page) und LMB-Drag (Manual-Position/Resize) haben
unterschiedliche Trigger, unterschiedliche Quellen, unterschiedliche
Releases. Ein gemeinsames Enum hätte sieben Varianten, jede mit eigener
disjunkter Felder-Menge — das ist die Definition von „Vermischung".
Belassen.

**5. `selection_slots_for` als pure Funktion** (5.1.3).
Drei Aufrufer (`MoveToNewPage`, `Move`, `SwapRange`) teilen sich die
Snippet aus der ursprünglichen `dispatch_move`. Nur einmal definiert,
dreimal aufgerufen.

## 6.4 Was NICHT in Phase 6 gehört

- Add-Dialog mit XMP-Filter, Multi-Path-Preview, Dry-Run-Anzeige —
  nachgelagert (Phase 7+).
- History-Dropdown (`fotobuch history`-Integration).
- Persistenter Thumbnail-Cache auf Disk.
- Snap-to-Grid im Manual-Mode.
- Rotations-Handle (Manual-Slot drehen) — Typst unterstützt Rotation, aber
  es ist kein UX-Konzept-Punkt.
- Drag von Fotos aus Pool in andere Pool-Gruppen (Re-Grouping).
- Konfigurierbare Toast-TTL / Bell-Sounds.

