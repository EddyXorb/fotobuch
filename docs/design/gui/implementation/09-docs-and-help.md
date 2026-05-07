# 09 · Documentation & In-App Help — Plan

## Assumptions

- `egui_commonmark` crate available for markdown rendering in egui.
- GUI markdown files are new — no existing docs to migrate.
- The mdBook toolchain (`docs/book/`) already builds and deploys.
- `FbTheme::ACCENT` (`#e08840`) is the golden color used for glow highlight.

---

## Approach

Two parallel tracks:

**A — Extend the mdBook** with a GUI chapter (static, searchable, public docs site).

**B — In-app help window** (F1 / `?`) that renders the same markdown with interactive extras:
lens mode (hover-to-explain) and keyword chips that glow the matching widget.

Only the GUI markdown is embedded in the app. CLI docs stay in the book only.

---

## 1 · mdBook: GUI chapter

New files under `docs/book/src/gui/`:

| File | Content |
|------|---------|
| `overview.md` | Conceptual layout — panels, workflow |
| `central-panel.md` | Canvas, zoom, page HUD, drop zones |
| `pool-panel.md` | Photo pool, drag sources |
| `toolbar.md` | Zoom, mode, project controls |
| `nav-panel.md` | Page nav strip |
| `keyboard.md` | All keyboard shortcuts |

Add a "GUI" section to `SUMMARY.md` pointing at these files.

---

## 2 · `HelpState` in `InteractionState`

New file `gui/state/help.rs`:

```rust
pub struct HelpState {
    pub open: bool,
    pub lens_active: bool,
    /// Written unconditionally by any documentable widget each frame.
    /// Contains the widget's slug and its screen rect.
    pub hovered_widget: Option<(&'static str, egui::Rect)>,
    /// Set by clicking a keyword chip in the help window.
    /// Cleared when the help window closes or the widget is clicked again.
    pub highlighted: Option<&'static str>,
}
```

Add `pub help: HelpState` to `InteractionState`.

`hovered_widget` is reset to `None` at the start of each frame (before any draw call), then
written by whichever documentable widget the pointer is over last. No lens-mode guard —
always written, free to use for other features later.

---

## 3 · Widget registration

Each documentable widget adds **one unconditional line** after its allocation:

```rust
// example in draw pool panel:
interaction.help.hovered_widget =
    Some(("panel-pool", panel_rect));
```

No trait, no registry, no `ctx.data`. The last writer each frame wins — natural priority
(topmost/last-drawn widget, which is correct for overlapping cases).

### Slug list (`HELP_SLUGS`)

Defined once in `gui/app/help.rs` (new file):

```rust
pub const HELP_SLUGS: &[&str] = &[
    "panel-pool",
    "panel-nav",
    "toolbar",
    "central-hud",
    "central-dropzone",
    "central-page",
];
```

Used by: the keyword pre-pass (§4) and any future tooling.

---

## 4 · Keyword → widget chip mapping

In `gui/app/help.rs`:

```rust
/// (exact phrase in markdown, slug)
pub static WIDGET_KEYWORDS: &[(&str, &str)] = &[
    ("Pool panel",    "panel-pool"),
    ("page HUD",      "central-hud"),
    ("drop zone",     "central-dropzone"),
    ("nav strip",     "panel-nav"),
    ("zoom slider",   "toolbar"),
];
```

Before handing markdown text to `egui_commonmark`, run a pre-pass that replaces each
keyword phrase with a custom `[[chip:slug]]` token. The custom renderer maps these tokens
to small clickable label widgets. Clicking one sets `interaction.help.highlighted = Some(slug)`.
The mdBook sees the original clean markdown — no special syntax leaks into the static site.

---

## 5 · Help window (`gui/app/widgets/help_window.rs`)

```
┌─────────────────────────────────────────────────┐
│  Help   [● Lens]                           [✕]  │
├──────────────┬──────────────────────────────────┤
│ Overview     │                                  │
│ Central panel│   <rendered markdown>            │
│ Pool panel   │   with clickable keyword chips   │
│ Toolbar      │                                  │
│ Keyboard     │                                  │
└──────────────┴──────────────────────────────────┘
```

- **Sidebar**: chapter list built from a static `&[(&str, &str)]` (title, embedded markdown).
  Clicking a chapter updates the content area.
- **Read mode** (default): content area shows the selected chapter.
- **Lens mode** (toggle `[● Lens]`): content area auto-replaces each frame with the doc
  section whose slug matches `interaction.help.hovered_widget`. The sidebar entry for
  that chapter is highlighted. Moving the mouse to a non-documented area keeps the last
  shown content (no flicker).
- **Keyword chips**: rendered inline in the text. Clicking sets `highlighted`; clicking
  again clears it. Highlighted chip gets accent color border.
- Opened by `F1` or a `?` button in the toolbar. State persists across opens/closes.

---

## 6 · Golden glow highlight

When `interaction.help.highlighted == Some(slug)` and a widget matches that slug, it draws
a pulsing glow ring on top of its normal render:

```
outer_alpha = 0.25 + 0.15 * sin(time * 5.0)
inner_alpha = 0.45 + 0.20 * sin(time * 5.0)

painter.rect_stroke(rect.expand(4.0), radius, Stroke::new(2.0, ACCENT @ outer_alpha))
painter.rect_stroke(rect.expand(1.5), radius, Stroke::new(1.5, ACCENT @ inner_alpha))
```

Driven by `ctx.request_repaint()` while `highlighted` is set.
Snaps off immediately when `highlighted` is cleared.

---

## File changes

| File | Action |
|------|--------|
| `docs/book/src/gui/*.md` | **new** — 6 GUI doc pages |
| `docs/book/src/SUMMARY.md` | add GUI section |
| `gui/state/help.rs` | **new** — `HelpState` struct |
| `gui/state.rs` | add `mod help; pub use help::HelpState;`, add `help` field to `InteractionState` |
| `gui/app/help.rs` | **new** — `HELP_SLUGS`, `WIDGET_KEYWORDS`, markdown embed + pre-pass |
| `gui/app/widgets/help_window.rs` | **new** — help window widget |
| `gui/app/widgets.rs` | wire up `help_window`, handle F1 key |
| per-panel draw files | add one `interaction.help.hovered_widget = Some((slug, rect))` line |
| `Cargo.toml` (gui) | add `egui_commonmark` dependency |

## Out of scope

- CLI docs in the app.
- Search within the help window.
- Localisation.
- Animated first-run tour.
