# Fotobuch · Central Panel — Implementation Spec

This document describes the redesigned **central panel** (the column where
fotobuch pages are stacked) for implementation in **Rust + egui**.

It is paired with `variation-3.jsx` and `fotobuch-shared.jsx` from the design
mock — those are the source of truth for spacing, colors, and states. **The
React code is a visual reference, not a code template.** Re-implement
semantically using egui idioms (`Ui::vertical`, `Frame`, `Response::hovered`,
`ui.dnd_drop_zone`, etc.).

---

## 1. Design intent ("Whisper")

> The fotobuch page is the hero. Chrome whispers; it never overlays the
> page or competes with it.

- The **page itself has zero overlay** — no badges, no buttons, no labels
  ever sit on top of the page graphic.
- All per-page UI lives **below** the page in a small HUD strip.
- The HUD is **calm by default** (greyed, ~55% opacity) and **brightens on
  hover** of the page row.
- The **only way to add a new page** is by **dragging a photo from the
  left filmstrip onto a drop zone** that sits centered between two
  adjacent pages. There is no "+" click button.

---

## 2. Layout

```
┌─────────── central panel ───────────┐
│                                     │
│       ┌──────────────────┐          │  page (centered)
│       │                  │          │
│       │    fotobuch      │          │
│       │     page         │          │
│       │                  │          │
│       └──────────────────┘          │
│             P00  •                  │  HUD (idle): "P00" + tiny mode dot
│                                     │
│        ╶╶╶ drop a photo ╶╶╶         │  drop zone (between pages only)
│                                     │
│       ┌──────────────────┐          │
│       │      page 1      │          │
│       └──────────────────┘          │
│             P01  ✦ AUTO  ↻ ✕        │  HUD (hovered): expanded
│                                     │
│        ╶╶╶ drop a photo ╶╶╶         │
│                                     │
│       ┌──────────────────┐          │
│       │      page 2      │          │
│       └──────────────────┘          │
│             P02  •                  │
│                                     │
└─────────────────────────────────────┘
```

- Vertical stack of `[ Page → HUD ] · DropZone · [ Page → HUD ] · …`
- DropZone appears **only between** two pages — never above the first or
  below the last.
- Everything is horizontally **centered** in the available width.
- Outer padding: `36px` top, `60px` bottom, `0px` horizontal.

---

## 3. Tokens

Use a single struct (e.g. `FbTheme`) with these values:

| Token        | Value      | Purpose                              |
|--------------|------------|--------------------------------------|
| `bg`         | `#1f2024`  | Central panel background             |
| `panel`      | `#26282d`  | (Side panels — not used here)        |
| `stroke`     | `#3a3d44`  | Borders, dividers                    |
| `stroke_hi`  | `#52565f`  | Hover/raised border                  |
| `text`       | `#e7e6e2`  | Primary text                         |
| `text_dim`   | `#9ea2ab`  | Secondary / button labels            |
| `text_mute`  | `#6b6f78`  | HUD labels at rest                   |
| `accent`     | `#e08840`  | Primary action / drop-zone active    |
| `auto`       | `#6aa9ff`  | Auto mode (cool blue)                |
| `manual`     | `#c8b18a`  | Manual mode (warm tan)               |
| `danger`     | `#d97777`  | Delete button                        |
| `paper`      | `#f4f1ec`  | The page itself                      |

Type:
- `ui` — proportional sans (`Inter` or system)
- `mono` — monospace (`JetBrains Mono` or fallback) — used for `P00` and
  mode label text

Sizes:
- HUD label / mode pill text: **10–11 px**, mono, letter-spacing ~0.5
- HUD row height: **24 px**
- Drop zone height: **44 px**
- Drop zone width: **520 px** (or matches the largest page width)

---

## 4. Components

### 4.1 PageBlock

```
┌────────────────────────┐
│       fotobuch page    │   ← FbPage (existing rendering)
└────────────────────────┘
        [ HUD strip ]        ← V3PageHud
```

- `gap` between page and HUD: **10 px**
- Page is rendered by your existing collage renderer.
- The whole block tracks a single `hovered` state — entering or leaving
  the bounding box toggles HUD expansion. In egui:
  ```rust
  let resp = ui.allocate_response(size, Sense::hover());
  let hovered = resp.hovered();
  ```

### 4.2 PageHud (the strip below each page)

Always centered, horizontal layout, gap **10 px**:

| Element        | Idle state                   | Hover state                              |
|----------------|------------------------------|------------------------------------------|
| `P00` label    | mono 10.5px, `text_mute`     | same, but parent opacity 1.0             |
| Mode dot/pill  | **10 px circle**, full color | **expands** into pill `✦ AUTO` (or `✋ MANUAL`) |
| Action buttons | hidden (opacity 0)           | fade in: `↻ Regenerate`, `✕ Delete`     |

- Whole strip opacity: `0.55` idle → `1.0` hover (animate ~150 ms)
- Action buttons appear with a small `translateX(-4px → 0)` slide.

#### Mode dot/pill detail

Idle:
- Circle, `width=height=10px`, fill = `auto` or `manual` color.
- No border, no text.

Hover (expanded):
- Pill, height **18 px**, padding **0 8px**.
- Background: mode color at **~13% alpha** (`#6aa9ff22` / `#c8b18a22`).
- Border: 1 px, mode color at **~40% alpha** (`#6aa9ff66` / `#c8b18a66`).
- Text: mode color, mono **10 px**, weight 700, letter-spacing 0.6.
- Content: `✦ AUTO` for auto, `✋ MANUAL` for manual. (You can swap the
  glyphs for egui-friendly icons; the colors carry the meaning.)
- **Click** toggles the mode for that page.

#### Action buttons

- 22×22 px, transparent fill, 1 px `stroke` border, radius 4 px.
- Icon color: `text_dim` for `↻`; `danger` for `✕`.
- Visible only while the row is hovered.

### 4.3 DropZone (between pages)

The **only way to add a new page**.

- Width equals the width of the page above, height **44 px**, centered, vertical margin **14 px**.
- Border: **1.5 px dashed**, color `stroke` idle, `accent` while drag-over.
- Background: transparent idle, `accent` at ~8% alpha (`#e0884014`) while drag-over.
- Content: small photo icon + text, both centered:
  - Idle: `"Drop a photo here to add a new page"` in `text_mute`.
  - Drag-over: `"Release to add new page"` in `accent`.
- **No click handler.** The drop zone responds only to drag-and-drop.

In egui, use `egui::Area` or a custom drop target — listen for
`ui.input(|i| i.pointer.any_released())` combined with a drag payload, or
use `ui.dnd_drop_zone::<Photo, _>(frame, |ui| { … })` if you have egui's
DnD support enabled.

#### State machine (per drop zone)

```
Idle ──drag enters──▶ Active
Active ──drag leaves──▶ Idle
Active ──drop──▶ insert new page at this index, with the dropped photo
```

When a photo is dropped:
1. Insert a new page at index `i+1` (if drop zone is between page `i` and
   page `i+1`).
2. The new page contains the dropped photo as its first slot.
3. Mode of the new page defaults to **Auto**.

---

## 5. Per-page state

```rust
struct Page {
    mode: PageMode,           // Auto | Manual
    photos: Vec<PhotoRef>,
    layout: Layout,           // produced by the auto layouter
    // ... existing fields
}

enum PageMode { Auto, Manual }
```

HUD actions map to existing app commands:

| HUD action | App command                           |
|------------|---------------------------------------|
| Mode pill click | toggle `Auto ↔ Manual` for this page  |
| `↻` Regenerate  | rerun the auto layouter for this page |
| `✕` Delete      | remove this page                      |

---

## 6. Animation / timings

All transitions: **150 ms**, ease-out. egui doesn't have CSS transitions —
emulate with frame-driven lerping on:

- `hud_opacity: f32` (0.55 ↔ 1.0)
- `pill_width: f32` (10 px ↔ measured pill width)
- `actions_opacity: f32` (0.0 ↔ 1.0)
- `actions_offset_x: f32` (-4 px ↔ 0 px)
- `dropzone_border_color`, `dropzone_bg` lerping toward target

Use `ui.ctx().request_repaint()` while a transition is in flight.

---

## 7. What NOT to do

- **No badge or label on top of the page graphic.** Ever.
- **No `+` click button** for inserting pages. Drop-only.
- **No frame, card, or border around the page** — page sits directly on
  the dark background, so the eye reads page-as-figure.
- **No per-page toolbar that's always visible** — actions appear only
  on hover.

---

## 8. Reference files

- `variation-3.jsx` — full visual reference (React mock).
- `fotobuch-shared.jsx` — color/type tokens (`FB` object) and the page
  rendering primitives.
- Screenshot of artboard "03 · Whisper" in the design canvas.

When prompting the LLM:

> Implement the central panel of the Fotobuch app in Rust + egui.
> Match the design described in `central-panel-spec.md`. Use the React
> source `variation-3.jsx` and `fotobuch-shared.jsx` as visual reference
> only — translate the layout, tokens, states and behavior into idiomatic
> egui. Do not port the React code literally.
