# 08 · Central Panel Polish — Plan

## Assumptions

- `DeletePages` and `RebuildPages` already exist in `BackgroundTask` — wire up directly.
- Pool drag (`DragSource::Pool`) is already trackable via `interaction.drag.active`.
- egui has no built-in lerp/animation — drive with frame-delta lerp + `request_repaint()`.
- Drop zone inserts at `at_position = i + 1` (zero-based) between page `i` and `i+1`.

## Steps

### 1 · Theme struct
- Add `gui/app/widgets/central_panel/theme.rs` with `FbTheme` constants matching spec §3.
- Re-export from `central_panel.rs`. Use throughout (no more color literals in this panel).

### 2 · Per-page HUD animation state
- Add `gui/state/page_hud.rs`:
  ```
  pub struct PageHudAnim {
      pub opacity: f32,       // 0.55 ↔ 1.0
      pub pill_width: f32,    // 10.0 ↔ expanded
      pub actions_alpha: f32, // 0.0 ↔ 1.0
      pub actions_offset: f32,// -4.0 ↔ 0.0
  }
  ```
  Default: `opacity=0.55`, rest=collapsed.
- Add `pub page_hud: HashMap<usize, PageHudAnim>` to `InteractionState`.
- Lerp factor `0.15s`-equivalent: `delta = ctx.input(|i| i.unstable_dt).min(0.05); k = 1.0 - (-delta/0.15).exp()` (or just `* (delta / 0.15).min(1.0) * 10.0` — keep simple).

### 3 · Rewrite `draw_page.rs` → PageBlock + PageHud

**PageBlock** (`draw_page`):
- Allocate bounding rect for the whole block (page + 10px gap + 24px HUD).
- `resp = ui.allocate_response(block_size, Sense::hover())` → `hovered = resp.hovered()`.
- Update `PageHudAnim` targets based on `hovered`, advance lerp, call `request_repaint()` while in-flight.
- Render page image (unchanged from current `render_page_image`).
- Call new `draw_hud()`.

**draw_hud** (replaces `ui.label` + `[A]`/`[M]` button):
- Centered row, height 24px, gap 10px between elements, mono font.
- Apply `hud_anim.opacity` via `ui.set_opacity()` (or manual alpha multiply).
- `P{idx:02}` label in `text_mute`.
- Mode dot/pill: idle → 10px filled circle; hover → pill (background + border + text). Click → `SetPageMode`.
- Action buttons (only when `actions_alpha > 0`):
  - `↻` → `RebuildPages { pages: vec![page_idx] }`
  - `✕` (danger color) → `DeletePages { pages: vec![page_idx] }`
  - 22×22px, `stroke` border, radius 4px. Offset by `actions_offset`.
- Remove the old `ui.label("Page {i}")` and `ui.small_button("[A]"/"[M]")`.

### 4 · Rewrite `draw_new_page_slot.rs` → DropZone

- Remove the [+] square; replace with 44px-tall dashed-border band, width = page width above (or 520px fallback).
- Detect active Pool drag: `matches!(interaction.drag.active, ActiveDrag::Dragging(DragSource::Pool {..}))`.
- Detect drag-over: `row_rect.contains(ctx.input(|i| i.pointer.latest_pos().unwrap_or_default()))`.
- Lerp border color + background toward `accent` when active+hovered, toward `stroke`/transparent otherwise.
- On pointer release while drag-over + Pool drag active → emit `BackgroundTask::Place { photo_ids, dst: PlaceDst::NewPageAt(at_position) }` (or equivalent — check what PlaceDst supports; if not, add `InsertPageWithPhoto` task).
- No click handler.

### 5 · Update `draw_pages.rs`

- Add `ui.add_space(36.0)` at top, `ui.add_space(60.0)` at bottom.
- Keep `draw_new_page_slot` **before page 0** (already exists, keep it — cover check stays).
- Keep `draw_new_page_slot` **after every page** including the last.

### 6 · Central panel background color

- `CentralPanel::default()` currently inherits egui default. Set `Frame::new().fill(FbTheme::BG)` on the central panel.

## File changes

| File | Action |
|------|--------|
| `central_panel/theme.rs` | **new** — FbTheme color constants |
| `state/page_hud.rs` | **new** — PageHudAnim struct + impl Default |
| `state.rs` | add `page_hud` field + `mod page_hud; pub use page_hud::PageHudAnim` |
| `central_panel/draw_page.rs` | rewrite — PageBlock + PageHud, remove old label+button |
| `central_panel/draw_new_page_slot.rs` | rewrite — DropZone (dashed, pool-drag-only) |
| `central_panel/draw_pages.rs` | padding, drop-zone placement (between only) |
| `central_panel.rs` | add BG frame, re-export theme |

## Out of scope

- `PlaceDst::NewPageAt` — if not supported, add minimal task variant only.
- Animating the page image itself.
- Right-rail / filmstrip changes.
