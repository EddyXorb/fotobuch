# Phase 3: Commands + Background-Pipeline

**Ziel**: GUI-Aktionen führen Lib-Commands aus, alles non-blocking.

---

## 3.1 — Command-Dispatch

### `gui/task.rs`

```rust
pub enum BackgroundTask {
    RenderPages { pages: Vec<usize>, pixel_per_pt: f32 },
    SwapSlots { src_page: usize, src_slot: usize, dst_page: usize, dst_slot: usize, pixel_per_pt: f32 },
    MoveSlot  { src_page: usize, src_slot: usize, dst_page: usize, dst_slot: usize, pixel_per_pt: f32 },
    Undo { pixel_per_pt: f32 },
    Redo { pixel_per_pt: f32 },
}

pub enum BackgroundResult {
    PageRendered { page: RenderedPage, rasterize_duration: Duration, compile_duration: Duration },
    CommandDone  { new_state: ProjectState, dirty_pages: Vec<usize> },
    CommandFailed(String),
    Error(String),
}
```

### `gui/background.rs`

Jede Command-Variante:
1. Passenden Lib-Call: `execute_move(project_root, PageMoveCmd::Swap/Move {...})` bzw. `undo/redo(project_root, 1)`
2. `changed_state.unwrap_or_default()` → `CommandDone { new_state, dirty_pages }`
3. Intern `render_pages(dirty_pages, pixel_per_pt)` — bestehende Render-Logik als Hilfsfunktion extrahiert

Dirty-Pages-Berechnung:
- Swap/Move: `pages_modified` aus `PageMoveResult` (als `usize`)
- Undo/Redo: alle Seiten `0..new_state.layout.len()`

### `gui/app.rs`

`drain_results` erweitern:
- `CommandDone`: `project_state` + `derived` updaten, `page_textures` und `page_dirty` auf neue Seitenzahl anpassen, dirty_pages auf `true` setzen
- `CommandFailed`: Log + ggf. Toast (Phase 4)

`PendingCommand` (lokal in `gui/app/pending.rs`):
```rust
pub enum PendingCommand {
    Swap { src_page: usize, src_slot: usize, dst_page: usize, dst_slot: usize },
    Move { src_page: usize, src_slot: usize, dst_page: usize, dst_slot: usize },
    Undo,
    Redo,
}
```

`FotobuchApp::dispatch(cmds: Vec<PendingCommand>)`:
- Jedes `PendingCommand` → passender `BackgroundTask` senden
- Vor Swap/Move: `page_dirty[src_page]` + `page_dirty[dst_page]` = `true` (sofortiges visuelles Feedback)
- Vor Undo/Redo: alle `page_dirty` = `true`

---

## 3.2 — Swap/Move (gleiche Seite)

### `gui/state/drag.rs` (neues Submodul)

```rust
pub enum DragState {
    Idle,
    Dragging { src_page: usize, src_slot: usize, is_move: bool },
}
```

`GuiState` bekommt: `pub drag: DragState`

### Input-Handling in `input_handler.rs`

Signatur: `pub fn handle(state: &mut GuiState, ctx: &egui::Context) -> Vec<PendingCommand>`

Drag-Flow:
- `primary_down` über Slot (nicht bereits Dragging) → `DragState::Dragging { src_page, src_slot, is_move: M-Taste gehalten }`
- `primary_released` + `DragState::Dragging` + `hovered_slot` vorhanden + Ziel ≠ Quelle (auf gleicher Seite) → `PendingCommand::Swap` oder `Move`
- `Escape` → `DragState::Idle` (bestehende clear-Logik bleibt)

### Overlay in `draw_page.rs`

Während Drag:
1. **Ghost**: Semi-transparentes Rect am Cursor (gleiche Größe wie Slot — `slot_rect_on_screen`, dann verschoben um Cursor-Offset). `Color32::from_rgba_unmultiplied(100, 149, 237, 120)`.
2. **Ziel-Highlight**: `hovered_slot` anders einfärben: grün wenn `|ratio_src/ratio_dst - 1.0| < 0.05`, sonst rot.

Ratio: `slot.width_mm / slot.height_mm`

**Tests** (in `state/drag.rs`):
- `drag_idle_by_default`
- `ratio_same_returns_green`, `ratio_different_returns_red` (Hilfsfunktion `slot_ratio_similar(a, b) -> bool`)

---

## 3.3 — Loading-Overlay + Undo/Redo

### `GuiState`

```rust
pub page_dirty: Vec<bool>,   // true → Seite wird gerade neu gerendert
```

`GuiState::new`: `page_dirty: vec![false; num_pages]`

### `draw_page.rs`

Wenn `page_dirty[i]` und Textur vorhanden:
- Textur normal zeichnen
- `Color32::from_rgba_unmultiplied(200, 200, 200, 150)` Rect darüber
- Label „↻" mittig

### Input-Handling

`Ctrl+Z` → `PendingCommand::Undo`
`Ctrl+Y` / `Ctrl+Shift+Z` → `PendingCommand::Redo`

### `drain_results` bei `PageRendered`

`state.page_dirty[page_idx] = false` (nach `apply_rendered`)

---

## Akzeptanzkriterien

- Slot ziehen → Ghost erscheint, Ziel-Slot färbt sich grün/rot
- Drop → Seite lädt (grauer Overlay), neue Textur erscheint
- Ctrl+Z/Y funktioniert, alle Seiten zeigen Loading-Overlay
- `cargo test` grün (inkl. neue Unit-Tests)
