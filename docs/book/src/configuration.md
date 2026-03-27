# Configuration

Every project has a `{project-name}.yaml` file that controls the entire book.
You don't need to write this file from scratch — `fotobuch project new` creates
it with sensible defaults. You only edit the parts you want to change.

Run `fotobuch config` at any time to see the full resolved configuration
(including all defaults).

---

## The most important settings

Before your first `fotobuch build`, open the YAML and check these:

### Page count (`config.book_layout_solver`)

The **book layout solver** decides how your photos are distributed across pages.
It uses a mathematical optimisation (Mixed Integer Programming) that balances
three goals: keep pages evenly filled, keep photo groups together, and hit your
target page count.

| Field | Default | What it does |
|---|---|---|
| `page_target` | `12` | The page count the solver aims for. Set this to the number of pages you want your book to have. |
| `page_max` | `26` | The maximum page count the solver is allowed to use. Set this **higher** than `page_target` to give the solver more freedom — it may use extra pages if that produces a better layout. |
| `page_min` | `1` | The minimum page count. Usually fine to leave at `1`. |

**Example:** You want roughly 20 pages but you're okay with up to 24 if
the solver finds a better layout:

```yaml
config:
  book_layout_solver:
    page_target: 20
    page_max: 24
```

> **Tip:** If you have many photos per group and tight page limits, the solver
> might struggle. Increase `page_max` or `photos_per_page_max` to give it
> more room.

### Page dimensions and margins (`config.book`)

| Field | Default | What it does |
|---|---|---|
| `page_width_mm` | set at creation | Page width in millimetres (e.g. 297 for A4 landscape). This is the width of a **single page** or a **double-page spread**, depending on your layout. |
| `page_height_mm` | set at creation | Page height in millimetres. |
| `bleed_mm` | `3.0` | Extra area around each page that gets cut off by the printer. Most print services require 3 mm. |
| `margin_mm` | `0.0` | Inset from the page edge — photos won't extend into this zone. Set to `0` for edge-to-edge printing (photos bleed to the edge). Set to e.g. `10` for a white border around each page. |
| `gap_mm` | `5.0` | Space between photos on the same page. |
| `bleed_threshold_mm` | `3.0` | When `margin_mm` is `0`, photos that end up closer to the edge than this threshold are automatically extended into the bleed area so they don't leave a thin white strip. |

**Single pages vs. double-page spreads:** The `page_width_mm` you set at
project creation determines whether fotobuch treats each page as one side or as
a left-right spread. For example, if your print service expects double-page
spreads of 420 × 297 mm, set `--width 420 --height 297` when creating
the project. For single pages, use `--width 210 --height 297`.

### Cover (`config.book.cover`)

If you created your project with `--with-cover`, page 0 is the cover. The
most important cover settings:

| Field | Default | What it does |
|---|---|---|
| `active` | `false` | Enable or disable the cover page. |
| `front_back_width_mm` | — | Total width of front + back panel (without spine). |
| `height_mm` | — | Cover height. |
| `spine_text` | book title | Text on the spine. Set to `~` (null) for no spine text. |
| `spine_mode` | `auto` | `auto` calculates spine width from page count; `fixed` uses a fixed value. |
| `spine_mm_per_10_pages` | `1.4` | In auto mode: spine thickness per 10 inner pages. |

### Solver timeout (`config.book_layout_solver`)

| Field | Default | What it does |
|---|---|---|
| `search_timeout` | `30s` | Time limit for the layout optimisation. For large books (100+ photos), you may want to increase this to give the solver more time. |

---

## All configuration sections

The YAML has this structure:

```yaml
config:
  book:           # page dimensions, margins, bleed, cover
  book_layout_solver:  # photo-to-page distribution (page count, etc.)
  page_layout_solver:  # single-page layout (genetic algorithm tuning)
  preview:        # preview rendering options
```

### Book settings (`config.book`)

| Field | Default | What it controls |
|---|---|---|
| `title` | `"Untitled"` | Book title (used as spine text if no explicit spine text is set) |
| `page_width_mm` | set at creation | Page width in mm |
| `page_height_mm` | set at creation | Page height in mm |
| `bleed_mm` | `3.0` | Bleed area around each page (cut off by the printer) |
| `margin_mm` | `0.0` | Inset from the page edge |
| `gap_mm` | `5.0` | Space between photos on a page |
| `bleed_threshold_mm` | `3.0` | Edge proximity threshold for bleed extension |
| `dpi` | `300` | DPI for the final release PDF |

### Photo distribution (`config.book_layout_solver`)

These parameters control how the solver assigns photos to pages.

| Field | Default | What it controls |
|---|---|---|
| `page_target` | `12` | Target page count |
| `page_min` | `1` | Minimum page count |
| `page_max` | `26` | Maximum page count |
| `photos_per_page_min` | `1` | Minimum photos on any page |
| `photos_per_page_max` | `20` | Maximum photos on any page |
| `group_max_per_page` | `5` | Max distinct groups sharing a page |
| `group_min_photos` | `1` | Min photos from a group when it's split across pages |
| `search_timeout` | `30s` | Solver time limit |

### Cover settings (`config.book.cover`)

| Field | Default | What it controls |
|---|---|---|
| `active` | `false` | Enable cover page |
| `front_back_width_mm` | — | Cover panel width |
| `height_mm` | — | Cover height |
| `spine_mode` | `auto` | `auto` or `fixed` |
| `spine_mm_per_10_pages` | `1.4` | Spine thickness per 10 pages (auto mode) |
| `spine_width_mm` | — | Fixed spine width (fixed mode) |
| `spine_text` | book title | Text on the spine |
| `bleed_mm` | `3.0` | Cover bleed |
| `margin_mm` | `0.0` | Cover margin |
| `gap_mm` | `5.0` | Gap between cover photos |
| `bleed_threshold_mm` | `3.0` | Cover bleed threshold |

### Preview settings (`config.preview`)

| Field | Default | What it controls |
|---|---|---|
| `max_preview_px` | `800` | Longest edge of cached preview images |
| `show_filenames` | `true` | Show filename captions in preview PDF |
| `show_page_numbers` | `true` | Show page numbers in preview PDF |

### Page layout solver (`config.page_layout_solver`)

These control the genetic algorithm that arranges photos within a single page.
You rarely need to change these.

| Field | Default | What it controls |
|---|---|---|
| `population_size` | `50` | Individuals per island |
| `max_generations` | `500` | Max generations |
| `mutation_rate` | `0.3` | Probability of mutation |
| `islands_nr` | `4` | Number of parallel islands |

> Run `fotobuch config` for the complete list of fields and their current values.

---

## Example: a typical YAML

```yaml
config:
  book:
    title: "Italy 2024"
    page_width_mm: 420.0    # double-page spread
    page_height_mm: 297.0
    bleed_mm: 3.0
    margin_mm: 0.0
    gap_mm: 5.0
    cover:
      active: true
      front_back_width_mm: 594.0
      height_mm: 297.0
      spine_text: "Italy 2024"
  book_layout_solver:
    page_target: 20
    page_max: 24
    photos_per_page_max: 8
    search_timeout:
      secs: 60
      nanos: 0
```
