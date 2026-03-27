# Command Overview

All commands follow the pattern `fotobuch <command> [options]`.
Run `fotobuch --help` or `fotobuch <command> --help` for details,
or see the [Full Flag Reference](cli/reference-generated.md).

## Commands at a glance

| Command | What it does |
|---|---|
| `project new` | Create a new photobook project |
| `project list` | List all projects in the current repo |
| `project switch` | Switch to another project (checks out its Git branch) |
| `add` | Import photos or folders into the project |
| `remove` | Delete photos from the project entirely |
| `place` | Assign unplaced photos to pages |
| `unplace` | Remove photos from their page slots (they stay in the project) |
| `build` | Solve layout and render preview PDF |
| `build release` | Render final PDF at full resolution (300 DPI) |
| `rebuild` | Re-run the solver on specific pages |
| `page move` | Move photos between pages |
| `page swap` | Swap pages or slots |
| `page split` | Split a page at a slot |
| `page combine` | Merge pages together |
| `page info` | Show photo metadata for slots on a page |
| `page weight` | Set the area weight for one or more slots |
| `status` | Show project overview (or single-page detail) |
| `config` | Print the resolved configuration with all defaults |
| `history` | Show the project change log |
| `undo` | Undo the last N changes |
| `redo` | Redo N undone changes |

### `remove` vs. `unplace`

- **`remove`** deletes photos from the project. They are gone (unless you `undo`).
- **`unplace`** takes photos off their page but keeps them in the project. They
  become unplaced and can be re-placed with `fotobuch place`.

Use `remove --keep-files` if you want remove-like pattern matching but
unplace-like behaviour (photos stay, just lose their page assignment).

### `build` vs. `rebuild`

- **`build`** renders the PDF and only re-solves pages that changed since the
  last build. On the first run it solves everything.
- **`rebuild --page N`** forces the solver to re-optimize page N from scratch,
  even if nothing changed. Useful when you're not happy with a layout.
- **`rebuild --all`** re-solves every page.

---

## YAML configuration

Every project contains a `{name}.yaml` file. Most settings have sensible
defaults. Run `fotobuch config` to see the current values.

Here are the fields you're most likely to touch. The YAML nesting is shown
as-is (e.g. `book.gap_mm` means the `gap_mm` key inside the `config.book`
section).

### Book settings (`config.book`)

| Field | Default | What it controls |
|---|---|---|
| `title` | `"Untitled"` | Book title (used as spine text if no explicit spine text is set) |
| `page_width_mm` | `210` | Page width in mm |
| `page_height_mm` | `297` | Page height in mm |
| `bleed_mm` | `3` | Bleed area around each page (cut off by the printer) |
| `margin_mm` | `0` | Inset from the page edge — photos won't extend into this zone |
| `gap_mm` | `5` | Space between photos on a page |
| `dpi` | `300` | DPI for the final release PDF |

### Page distribution (`config.book_layout_solver`)

| Field | Default | What it controls |
|---|---|---|
| `page_target` | `32` | Target page count the solver aims for |
| `page_min` | `1` | Hard minimum page count |
| `page_max` | `48` | Hard maximum page count |
| `photos_per_page_min` | `2` | Minimum photos on any page |
| `photos_per_page_max` | `20` | Maximum photos on any page |
| `group_max_per_page` | `3` | Max distinct groups per page |
| `group_min_photos` | `2` | Min photos from a group if it's split across pages |

### Preview (`config.preview`)

| Field | Default | What it controls |
|---|---|---|
| `max_preview_px` | `800` | Longest edge of cached preview images |
| `show_filenames` | `true` | Show filename captions in preview PDF |
| `show_page_numbers` | `true` | Show page numbers in preview PDF |

### Cover (`config.book.cover`)

| Field | Default | What it controls |
|---|---|---|
| `active` | `false` | Enable cover page (page 0) |
| `front_back_width_mm` | — | Cover panel width (set via `--cover-width` on project creation) |
| `height_mm` | — | Cover height (set via `--cover-height`) |
| `spine_mode` | `auto` | `auto` grows spine with page count; `fixed` uses a fixed width |

### Example YAML snippet

```yaml
config:
  book:
    title: "Italy 2024"
    page_width_mm: 297.0
    page_height_mm: 210.0
    bleed_mm: 3.0
    margin_mm: 0.0
    gap_mm: 5.0
    dpi: 300.0
    cover:
      active: false
  book_layout_solver:
    page_target: 32
    page_min: 1
    page_max: 48
    photos_per_page_min: 2
    photos_per_page_max: 20
  preview:
    show_filenames: true
    show_page_numbers: true
    max_preview_px: 800
```

> The `page_layout_solver` section (genetic algorithm parameters) exists too but
> rarely needs tweaking. Run `fotobuch config` to see all fields.
