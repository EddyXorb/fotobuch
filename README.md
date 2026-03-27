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

<!-- USER: please finalize this section. Draft below based on your notes. -->

Photobooks should be made a certain way. `fotobuch` is an opinionated tool that supports
exactly one philosophy of what a great photobook looks like — and pursues it without
compromise.

**Why opinionated?** Every design decision in the layout algorithm reflects a deliberate
aesthetic stance: photos are not cropped, not distorted, not squeezed to fill a gap.
A photographer chooses a frame intentionally. `fotobuch` respects that choice.

Key principles:

- **Full automation, full control.** Let the solver do all the work, or step in and
  adjust any page manually. Both workflows are first-class.
- **Aspect ratios are sacred.** Every photo is shown exactly as it was shot — no
  cropping, no distortion. The solver finds a layout that fits, not one that forces.
- **Reading flow matters.** Photos are arranged so the eye moves naturally from top-left
  to bottom-right, mirroring how we read. The sequence of your story is preserved.
- **Weight your images.** Tell `fotobuch` which photos deserve more space. Important
  moments appear larger; supporting shots stay smaller. The balance is yours to set.
- **Groups stay together.** Photos from the same event or folder are treated as a unit.
  The book flows from one group to the next without mixing.
- **Tiny footprint.** A project is fully described by a single YAML file and a Typst
  source file. As long as your source photos stay where they are, the entire book takes
  almost no additional disk space.
- **Science-backed algorithms.** The layout engine is built on published research and
  novel unpublished extensions — the result is a solver that produces genuinely better
  layouts than off-the-shelf approaches. See [Technical Background](#technical-background).

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
# Example: key fields
page:
  width_mm: 297
  height_mm: 210
  bleed_mm: 3
  margin_mm: 10
  gap_mm: 3

solver:
  max_photos_per_page: 6
  mip_timeout_secs: 30

preview:
  dpi: 150

release:
  dpi: 300
  jpg_quality: 95
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

### Page layout solver — Genetic algorithm with DFS ordering

Single-page layout is solved by a genetic algorithm operating on *slicing trees* — a
data structure from academic layout research. Each individual in the population encodes
a complete binary partition of the page area.

**Novel contribution:** The standard slicing-tree crossover and mutation operators
assume photos can be placed in arbitrary order. `fotobuch` enforces reading order via
a depth-first traversal of the tree (DFS indexing). This requires a fundamentally
different mutator: instead of swapping arbitrary nodes, it swaps only leaves with
compatible aspect ratios, preserving the DFS sequence. This improvement is not described
in the literature and produces measurably better results in terms of reading-flow
coherence.

The population is evolved with island-model parallelism: independent sub-populations
evolve in parallel on separate threads, with periodic migration between islands.

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
