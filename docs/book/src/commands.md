# Command Overview

All commands follow the pattern `fotobuch <command> [options]`.
Run `fotobuch --help` or `fotobuch <command> --help` for a full flag listing,
or see the [Full Flag Reference](cli/reference-generated.md).

## Commands at a glance

| Command | What it does |
|---|---|
| `project new` | Create a new photobook project |
| `project list` | List all projects in the current directory tree |
| `project switch` | Switch the active project |
| `add` | Add photos or folders to the project |
| `remove` | Remove photos or groups from the project |
| `place` | Place unplaced photos into the layout |
| `unplace` | Remove photos from the layout (photos stay in the project) |
| `build` | Solve layout and render preview PDF (low DPI for faster browsing) |
| `build release` | Render final PDF at full resolution (300 DPI) |
| `rebuild` | Re-run the solver on specific pages |
| `page move` | Move or swap photos between pages |
| `page swap` | Swap pages or slots |
| `page split` | Split a page at a slot |
| `page combine` | Merge pages together |
| `page info` | Show photo metadata for slots on a page |
| `page weight` | Set the area weight for one or more slots |
| `status` | Show project overview or single-page detail |
| `config` | Print resolved configuration with all defaults |
| `history` | Show the project change log |
| `undo` | Undo the last N changes |
| `redo` | Redo N undone changes |

---

## Slot addressing

Many commands take a *slot address* to identify one or more photos on a page.

| Address | Meaning |
|---|---|
| `3` | All slots on page 3 |
| `3:2` | Slot 2 on page 3 |
| `3:2..5` | Slots 2 through 5 on page 3 |
| `3:2..5,7` | Slots 2–5 and slot 7 on page 3 |
| `4+` | New page inserted after page 4 (move destination only) |

Pages and slots are numbered from 0. Use `fotobuch status <page>` to see the
slot numbers for a specific page.

---

## YAML configuration

Every project contains a `fotobuch.yaml` file. Most settings have sensible
defaults; you only need to change what matters to you.

Run `fotobuch config` to see the current values including all defaults.

Key fields:

| Field | Default | Description |
|---|---|---|
| `title` | `"Untitled"` | Book title (also used as spine text) |
| `page_width_mm` | `210` | Page width in millimetres |
| `page_height_mm` | `297` | Page height in millimetres |
| `bleed_mm` | `3` | Bleed margin added around each page |
| `margin_mm` | `0` | Inner margin (safe zone away from the cut edge) |
| `gap_mm` | `5` | Gap between photos on a page |
| `dpi` | `300` | DPI for the final release PDF |
| `solver.page_target` | `32` | Target number of pages |
| `solver.page_min` | `1` | Minimum number of pages |
| `solver.page_max` | `48` | Maximum number of pages |
| `solver.photos_per_page_min` | `2` | Minimum photos per page |
| `solver.photos_per_page_max` | `20` | Maximum photos per page |
| `preview.max_preview_px` | `800` | Longest edge of cached preview images |
| `preview.show_filenames` | `true` | Show filename captions in preview |
| `cover.active` | `false` | Enable cover page |

The full YAML schema is documented in the source at
`docs/design/yaml-scheme.md`.
