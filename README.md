# fotobuch

<!-- Badges -->
[![CI](https://github.com/EddyXorb/fotobuch/actions/workflows/ci.yml/badge.svg)](https://github.com/EddyXorb/fotobuch/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/EddyXorb/fotobuch/branch/main/graph/badge.svg)](https://codecov.io/gh/EddyXorb/fotobuch)
[![Release](https://img.shields.io/github/v/release/EddyXorb/fotobuch)](https://github.com/EddyXorb/fotobuch/releases/latest)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

---

<!-- EXAMPLE IMAGE: replace with a side-by-side screenshot of a cover + first inner double page spread
     showing a real layout produced by fotobuch. Aim for something visually striking –
     similar in style to https://github.com/masse/collage-solver
     Suggested filename: docs/assets/example_spread.jpg -->
![Example photobook spread](docs/assets/example_spread.jpg)

---

## What is fotobuch?

Photobooks should be made a certain way. `fotobuch` is an opinionated tool that supports
exactly one philosophy of what a great photobook looks like.

**Why opinionated?** Every design decision in the layout algorithm reflects a deliberate
aesthetic stance: photos are not cropped, not distorted, not squeezed to fill a gap.
A photographer chooses a frame intentionally. `fotobuch` respects that choice.
There are no splashy frames, overlays, smileys, or other distracting objects in the photobook —
only the photos you chose to include, presented with care.

Further key principles:

- **Full automation, full control.** Let the solver do all the work, or step in and
  adjust any page manually.
- **Aspect ratios are sacred.** Every photo is shown exactly as it was shot — no
  cropping, no distortion. The solver finds a layout that fits, not one that forces.
- **Reading flow matters.** Photos are arranged so the eye moves naturally from top-left
  to bottom-right, mirroring how we read. The sequence of your story is preserved.
- **Weight your images.** Tell `fotobuch` which photos deserve more space. Important
  moments appear larger; supporting shots stay smaller. The balance is yours to set.
- **Groups stay together.** Photos from the same event or folder are treated as a unit.
  The book flows from one group to the next without mixing, unless you want it to.
- **Tiny footprint.** A project is fully described by a single YAML file and a Typst
  source file. As long as your source photos stay where they are, the entire book takes
  almost no additional disk space.
- **Science-backed algorithms.** The layout engine is built on published research and
  novel unpublished extensions — the result is a solver that produces genuinely better
  layouts than off-the-shelf approaches. See [Technical Background](#technical-background).

## Background

I am a passionate photographer. I document many of the things that happen around me. Taking a lot of pictures is easy, but it makes no sense for me to just shoot them without doing something with them later.
So every year I sit down and group, rate, post-process and print my photos. In my workflow, the printing is done by creating a photobook and that was always a pain: the commercial print services normally come with some in-house software solution that does not work well for me.
The pain points normally are:

- no option to export the photobook to pdf in full size
- bad/restricted automation options for placing the photos on a page
- to maintain a copy of the photobook you normally have to keep some proprietary files that contain all the photos you already have on your disk - wasted space
- even if the automations respect the chronology of the photos when distributed across pages, they do not respect the chronological order within a page
- it is often not possible to tell the automatic solver which photos should be bigger than others
- bugs, crashes and loss of data have happened to me more than once

So I decided to solve these problems, and I am happy if it works also for you.

---

## Installation

**Requirements:** Rust (stable) — install via [rustup.rs](https://rustup.rs)

```bash
git clone https://github.com/EddyXorb/fotobuch.git
cd fotobuch
cargo build --release
# binary: ./target/release/fotobuch
```

Pre-built binaries for Linux and Windows are available on the
[Releases page](https://github.com/EddyXorb/fotobuch/releases/latest).

---

## Quick Start

**Recommended:** use [VS Code](https://code.visualstudio.com/) with the
[Typst Preview](https://marketplace.visualstudio.com/items?itemName=mgt19937.typst-preview)
extension so you see the layout update live. Alternatively, just keep a PDF viewer open.

```bash
# 1. Create a new project (page size in mm, e.g. A4 landscape)
fotobuch project new my-book --width 297 --height 210

# 2. Add photos (folders are treated as groups; timestamps in folder names set order)
fotobuch add /path/to/photos/2024-07-Italy
fotobuch add /path/to/photos/2024-08-Hiking

# 3. Build a preview (150 DPI, fast)
fotobuch build

# 4. Adjust the layout
fotobuch page swap 3 5            # swap pages 3 and 5
fotobuch page move 3:2 4          # move slot 2 from page 3 to page 4
fotobuch rebuild --page 6         # re-run solver on page 6 only
fotobuch undo                     # undo last change

# 5. Export final PDF (300 DPI, print-ready)
fotobuch build release
```

Full workflow and all commands: **[see the documentation](https://eddyxorb.github.io/fotobuch)**

---

## YAML Configuration

Every project is described by a `fotobuch.yaml` in the project directory.
Running `fotobuch config` prints the resolved configuration with all defaults applied.

```yaml
config:
  book:
    title: my-book
    page_width_mm: 297.0
    page_height_mm: 210.0
    bleed_mm: 3.0
    margin_mm: 10.0
    gap_mm: 3.0
    dpi: 300.0
  book_layout_solver:
    photos_per_page_min: 1
    photos_per_page_max: 6
    search_timeout:
      secs: 30
      nanos: 0
  preview:
    show_filenames: true
    show_page_numbers: true
    max_preview_px: 800
```

The layout itself is stored as a Typst (`.typ`) file alongside the YAML. Advanced users
can edit it directly and recompile with `typst compile fotobuch.typ fotobuch.pdf`.

Full configuration reference: **[see the documentation](https://eddyxorb.github.io/fotobuch)**

---

## Technical Background

### Git-based project history

Every change made by `fotobuch` (adding photos, rebuilding a page, moving slots) is
committed to a Git repository inside the project folder. You can inspect the full
history with `fotobuch history` or standard `git log`, and roll back any step with
`fotobuch undo`.

### Caching and DPI-accurate rendering

`fotobuch` maintains two image caches:

- **Preview cache** – downscaled to 150 DPI (configurable). Fast to build, used during
  layout iteration.
- **Release cache** – full resolution at 300 DPI (configurable). Built on demand for the
  final print PDF.

The Typst compiler embeds cached images, so the final PDF is fully self-contained and
ready to upload to print services such as Saal Digital.

### Incremental builds

Only pages that have changed since the last build are recomputed. A change is detected
by comparing image hashes and layout state. This makes iterative refinement fast even
for large books.

### PDF generation — Typst

The final photobook is rendered to PDF by [Typst](https://typst.app/), a modern
typesetting system that replaces LaTeX for programmatic document generation.
`fotobuch` emits a `.typ` source file describing every page, and Typst compiles it
into a print-ready PDF in (milli-)seconds.

Typst deserves a special mention here: what would have been a nightmare of fragile
LaTeX macros, broken package dependencies, and cryptic error messages turned out to be
a breeze. Typst's clean scripting model, fast compile times, and first-class support
for precise absolute positioning made it the ideal backend for a layout-heavy tool like
`fotobuch`. A big thank-you to the Typst team and community for building something
this good.

### Page layout solver — Genetic algorithm with exact gap handling

Single-page layout is solved by a genetic algorithm operating on *slicing trees*, a
data structure from academic layout research. The foundational algorithm is described in:

> O. Fan, *"Genetic Algorithm for Layout Optimization"*,
> [IEEE Xplore, 2012](https://ieeexplore.ieee.org/document/6266273).
> Many thanks to the author for this elegant foundation.

A big thank-you also to [@masse](https://github.com/masse) for
[collage-solver](https://github.com/masse/collage-solver), whose work was an inspiring
starting point and proof that slicing-tree approaches produce genuinely beautiful results.

Each individual in the population encodes a complete binary partition of the page area.
The population is evolved with island-model parallelism: independent sub-populations
evolve in parallel on separate threads, with periodic migration between islands.

**Novel contribution — exact gap computation in O(n):**
The original algorithm approximates the inter-photo gap (beta) or recomputes it in
O(n³) per fitness evaluation. `fotobuch` derives an exact closed-form solution: given
a slicing tree, the precise gap that fills the page without any overlap or leftover
space can be computed via an *affine vector-space transformation* of the tree's size
expressions. This reduces complexity from O(n³) to **O(n)** per evaluation — a
significant speedup for pages with many photos — while guaranteeing pixel-accurate
placement. This formulation does not appear in the literature.

**Reading-order preservation via DFS indexing:**
Photos are assigned to tree leaves in depth-first order, ensuring the visual reading
sequence (top-left to bottom-right) matches the chronological order of the input.
This requires a different mutation strategy: instead of swapping arbitrary nodes, the
mutator exchanges only leaves with compatible aspect ratios, preserving the DFS sequence.

### Book layout solver — Mixed Integer Programming

Assigning photos to pages (how many photos go on which page, and which photos belong
together) is formulated as a Mixed Integer Program and solved with
[HiGHS](https://highs.dev/). The objective balances page fill, group coherence, and
user-defined photo weights. For large books the problem is automatically decomposed into
independent sub-problems to stay within the solver time budget.

---

## License

`fotobuch` is licensed under the [GNU Affero General Public License v3.0](LICENSE).

For commercial use (e.g. integrating fotobuch into a paid service or product), please
[contact me](mailto:YOUREMAIL@example.com) to discuss a commercial license.
