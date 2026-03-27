# Configuration

Every project has a `{project-name}.yaml` file that controls the entire book.
You don't need to write this file from scratch — `fotobuch project new` creates
it with sensible defaults. You only edit the parts you want to change.

Run `fotobuch config` at any time to see the full resolved configuration
(including all defaults).

For a quick overview of the most important settings to check before your first
build, see [Step 2 in Your First Book](quickstart.md#step-2--review-the-configuration).

> **Tip:** If the solver produces poor results, the most common fixes are:
> increase `search_timeout` (more time), or disable `enable_local_search`.
> You may also want to tweak the solver weights according to your needs
> (e.g. increase `weight_split` if you want groups to be respected more
> strictly).

---

## Full reference

The YAML has this structure:

```yaml
config:
  book:                  # page dimensions, margins, bleed, cover
    cover:               # cover-specific settings (nested inside book)
  book_layout_solver:    # photo-to-page distribution
  page_layout_solver:    # single-page layout (genetic algorithm)
    weights:             # fitness function weights (nested)
  preview:               # preview rendering options
```

All fields below are optional unless marked otherwise. Defaults are applied
automatically for any field you don't set.

---

### `config.book` — Page dimensions and layout

| Field | Default | Description |
|---|---|---|
| `title` | `"Untitled"` | Book title. Used as the default spine text on the cover. |
| **`page_width_mm`** | `210.0` | **Page width in mm.** Set at project creation with `--width`. For double-page spreads, use the combined width (e.g. `420`). |
| **`page_height_mm`** | `297.0` | **Page height in mm.** Set at project creation with `--height`. |
| **`bleed_mm`** | `3.0` | **Bleed area in mm** added around each page. Cut off by the printer. Most services require 3 mm. |
| **`margin_mm`** | `0.0` | **Minimum inset from the page edge.** `0` = edge-to-edge (photos may bleed). `> 0` = white border (bleed extension is disabled). |
| **`gap_mm`** | `5.0` | **Space in mm between photos** on the same page. |
| `bleed_threshold_mm` | `3.0` | Only active when `margin_mm` is `0`. If a photo's edge is closer to the page edge than this value, the layout is scaled so the photo extends fully into the bleed area. Prevents thin white strips at the page edge after cutting. |
| `dpi` | `300.0` | DPI for the final release PDF. Controls the resolution of cached images used in `fotobuch build release`. |

---

### `config.book.cover` — Cover page

All cover fields are optional. The cover is inactive by default. Enable it by
setting `active: true` and providing dimensions.

| Field | Default | Description |
|---|---|---|
| **`active`** | `false` | **Enable the cover.** When `true`, the first layout entry (page 0) becomes the cover page. |
| **`front_back_width_mm`** | `0.0` | **Total width of front + back panel combined, without the spine.** Required when `active: true`. |
| **`height_mm`** | `0.0` | **Cover height in mm.** Required when `active: true`. |
| `mode` | `free` | **Cover layout mode.** Controls how page 0 is solved. `free` = GA solver optimises freely (default). All other modes use a deterministic solver and bypass the GA. See [Cover modes](#cover-modes) below. |
| `spine_clearance_mm` | `5.0` | Gap in mm between the photo edge and the spine for `front`, `back`, and `split` modes. Ignored for `spread` modes. |
| **`spine_text`** | book title | **Text on the spine.** Set to `~` (null) for no text. Font size is auto-calculated from the spine width (max 80% of spine width). |
| `spine_mode` | `auto` | Spine width mode — see below. |
| `spine_mm_per_10_pages` | `1.4` | **Auto mode only.** Spine thickness per 10 inner pages. Spine width = `(inner_pages / 10) * spine_mm_per_10_pages`. In auto mode the spine width affects the total cover canvas width that the solver uses. |
| `spine_width_mm` | — | **Fixed mode only.** A fixed spine width in mm. In fixed mode the spine does **not** affect the cover canvas width in the solver — it is only used by the template for display and text sizing. |
| `bleed_mm` | `3.0` | Bleed for the cover page (independent from inner-page bleed). |
| `margin_mm` | `0.0` | Margin for the cover page. Same behaviour as the inner-page margin. |
| `gap_mm` | `5.0` | Gap between photos on the cover. |
| `bleed_threshold_mm` | `3.0` | Bleed threshold for the cover. Same behaviour as inner pages. |

#### Cover modes

When `mode` is not `free`, the GA solver is bypassed and slot positions are
calculated deterministically from the cover geometry.  A warning is printed if
the number of photos on the cover does not match what the mode expects.

| Mode | Photos | Behaviour |
|------|--------|-----------|
| `free` | any | GA solver optimises freely (default) |
| `front` | 1 | Photo on the front panel, aspect ratio preserved and centred |
| `front-full` | 1 | Photo fills the entire front panel (may crop) |
| `back` | 1 | Photo on the back panel, aspect ratio preserved and centred |
| `back-full` | 1 | Photo fills the entire back panel (may crop) |
| `spread` | 1 | Photo spans the full spread (over spine), aspect ratio preserved and centred |
| `spread-full` | 1 | Photo fills the full spread (may crop) |
| `split` | 2 | Slot 0 → front, slot 1 → back, aspect ratio preserved and centred |
| `split-full` | 2 | Slot 0 → front, slot 1 → back, each half fully filled (may crop) |

**Workflow example — single photo on the front:**

```bash
# 1. Place the photo onto the cover
fotobuch place cover.jpg --into 0

# 2. Set the mode in the YAML
#    cover:
#      mode: front

# 3. Rebuild only the cover
fotobuch rebuild --page 0
```

**Workflow example — panorama across full spread:**

```bash
fotobuch place panorama.jpg --into 0
# cover: { mode: spread-full }
fotobuch rebuild --page 0
```

**Spine modes explained:**

- **`auto`** (default): Spine width is calculated from the number of inner pages.
  The formula is `spine_width = (inner_pages / 10) * spine_mm_per_10_pages`.
  The spine width is **added to `front_back_width_mm`** to form the total cover
  canvas — meaning the solver accounts for the spine. Use this when your print
  service calculates spine from page count (most common).

- **`fixed`**: You provide `spine_width_mm` directly. The spine is **not added**
  to the canvas width — the solver uses `front_back_width_mm` as-is. The fixed
  spine width is only used by the template for positioning the spine text. Use
  this when you already know the exact spine width from your print service.

---

### `config.book_layout_solver` — Photo-to-page distribution

The book layout solver distributes your photos across pages. It first runs a
Mixed Integer Program (MIP) to find a globally optimal assignment, then refines
it with a local search that evaluates actual layout quality per page.

| Field | Default | Description |
|---|---|---|
| **`page_target`** | `12` | **Target number of pages.** The solver tries to hit this count. This is the most important solver setting. |
| `page_min` | `1` | Hard minimum number of pages. |
| **`page_max`** | `26` | **Hard maximum number of pages.** Setting this above `page_target` gives the solver room to add pages when that improves layout quality. |
| `photos_per_page_min` | `1` | Minimum number of photos on any single page. |
| `photos_per_page_max` | `20` | Maximum number of photos on any single page. |
| `group_max_per_page` | `5` | Maximum number of different groups that may share a single page. Lower values keep groups more separated. |
| `group_min_photos` | `1` | When a group is split across two pages, each part must have at least this many photos. Prevents a single "orphan" photo appearing alone on the next page. |
| `weight_even` | `1.0` | MIP objective weight for even photo distribution across pages. Higher = more uniform page fill. |
| `weight_split` | `10.0` | MIP objective weight penalising group splits. Higher = groups are less likely to be split across pages. |
| `weight_pages` | `5.0` | MIP objective weight penalising deviation from `page_target`. Higher = result stays closer to the target. |
| **`search_timeout`** | `30s` | **Time budget for the entire solver** (MIP + local search). Increase for large books. YAML format: `{secs: 60, nanos: 0}`. |
| `enable_local_search` | `true` | Whether to run the local search after the MIP. The local search shifts page boundaries to improve per-page layout quality. |
| `mip_rel_gap` | `0.01` | Relative optimality gap for the MIP solver (0.0 = exact, 0.01 = accept solutions within 1% of optimal). Tightening this rarely helps and increases solve time. |
| `max_photos_for_split` | `300` | When the total photo count exceeds this, the problem is automatically decomposed into smaller sub-problems solved sequentially. This avoids MIP timeouts on very large books. |
| `split_group_boundary_slack` | `5` | When splitting into sub-problems, the split point may deviate by this many photos from the ideal boundary to prefer splitting at a group boundary. |
| `max_coverage_cost` | `0.95` | **(Currently unused — will be removed in a future version.)** Was intended as a threshold for the local search to identify "bad" pages, but is not read by the solver. |

---

### `config.page_layout_solver` — Single-page layout (genetic algorithm)

The page layout solver arranges photos within a single page using a genetic
algorithm with island-model parallelism. These are advanced tuning parameters —
the defaults work well for most cases.

| Field | Default | Description |
|---|---|---|
| `seed` | `42` | Random seed for the genetic algorithm. Change this to get a different layout for the same input. `fotobuch rebuild` changes the seed automatically. |
| `population_size` | `750` | Number of individuals (candidate layouts) per island. Larger = better results but slower. |
| `max_generations` | `100` | Maximum number of generations the algorithm runs. |
| `mutation_rate` | `0.3` | Probability that an individual is mutated per generation. |
| `crossover_rate` | `0.7` | Probability that two individuals are recombined per generation. |
| `elite_count` | `20` | Number of best individuals carried over unchanged to the next generation. |
| `no_improvement_limit` | `15` | Stop early if no improvement is found for this many generations. Set to `~` (null) to disable early stopping. |
| `enforce_order` | `true` | Enforce chronological reading order (top-left to bottom-right) on each page. When `true`, photos are arranged so earlier photos appear before later ones in natural reading direction. Set to `false` if you don't care about photo order and want the solver to optimise purely for visual quality — this often produces tighter layouts. |
| `islands_nr` | CPU cores | Number of independent populations evolved in parallel. Defaults to the number of available CPU cores. |
| `islands_migration_interval` | `5` | Generations between migration events (best individuals are copied between islands). |
| `islands_nr_migrants` | `2` | Number of individuals migrated per island per migration event. |

#### `config.page_layout_solver.weights` — Fitness function

The fitness function evaluates how good a single-page layout is. It combines
three cost components, each multiplied by its weight. Lower cost = better layout.

| Field | Default | Description |
|---|---|---|
| `w_coverage` | `1.0` | Weight for canvas coverage cost. Penalises unused white space on the page. This is the dominant term. |
| `w_size` | `0.2` | Weight for size distribution cost. Penalises photos that deviate from their target size (determined by their `area_weight`). |
| `w_barycenter` | `0.0` | Weight for barycenter centering cost. Penalises layouts whose visual centre of mass is far from the page centre. Disabled by default (`0.0`). |

---

### `config.preview` — Preview rendering

All preview overlay settings are automatically suppressed in `build release`.

| Field | Default | Description |
|---|---|---|
| `show_filenames` | `false` | Show the photo filename as a caption on each photo. Useful for identifying photos when adjusting the layout. |
| `max_preview_px` | `800` | Maximum pixel size (longest edge) of cached preview images. Lower = faster builds, less disk space, blurrier preview. |
| `show_borders` | `true` | Show red bleed border and blue margin border overlays on each page. |
| `show_slot_info` | `true` | Show slot address and area weight on each photo (e.g. `3:2 (1.5)`). |

---

### `config.book.appendix` — Photo index

The appendix is a compact photo index appended at the end of both the preview
and release PDFs, listing every photo with its group, timestamp, and a
page-position reference.

| Field | Default | Description |
|---|---|---|
| `active` | `false` | Enable the photo index. |
| `columns` | `7` | Number of columns in the listing. |
| `ref_mode` | `"positions"` | Reference style: `"positions"` (page.slot, e.g. `2.3`) or `"counter"` (sequential number badge on each photo). |
| `page_separator` | `false` | Show a page-number header between pages in the listing. |
| `strip_timestamps` | `true` | Strip leading ISO timestamps from filenames in the listing. |
| `label_title` | `"Photo Index"` | Title text of the appendix. |
| `label_page` | `"Page"` | "Page" label used in the cross-reference legend and page separators. |
| `date_format` | `"{day}. {month} {year} {hour}:{min}"` | Format string for timestamps. Placeholders: `{day}`, `{month}`, `{year}`, `{hour}`, `{min}`. |
| `date_months` | `["Jan", …, "Dec"]` | Month abbreviations (12 entries, January–December). |

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
      spine_mode: auto
      spine_mm_per_10_pages: 1.4
      spine_text: "Italy 2024"
    appendix:
      active: true
      label_title: "Photo Index"
      label_page: "Page"
  book_layout_solver:
    page_target: 20
    page_max: 24
    photos_per_page_max: 8
    search_timeout:
      secs: 60
      nanos: 0
  preview:
    show_filenames: true
    show_borders: true
    show_slot_info: true
    max_preview_px: 800
```
