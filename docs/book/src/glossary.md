# Glossary

Domain-specific terms used throughout fotobuch. This is the authoritative definition source — all code, documentation, and communication should use these terms consistently.

---

## Data and Storage

| Term             | Definition                                                                                                                                                             |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Vault**        | A folder on disk that holds one or more [projects](#project) as a single git repository.                                                                              |
| **Project**      | A single photo book — one YAML file (photo list, layout, configuration) plus a Typst template — stored alongside other projects in a [vault](#vault).                  |
| **Template**     | The Typst file that defines the visual design of a [project](#project) — page size, fonts, spacing, and how [slots](#slot) are rendered in the PDF.                    |
| **Photo**        | A single image file imported into a [project](#project), belonging to exactly one [photo group](#photo-group).                                                         |
| **Photo Group**  | A named collection of [photos](#photo), typically corresponding to a source folder.                                                                                    |
| **Photo Weight** | A numeric value on a [photo](#photo) that controls how much space the layout gives it relative to other photos on the same [page](#page).                              |
| **History**      | The complete record of all changes to a [project](#project) — stored as git commits, shared between GUI and CLI, and the basis for [undo and redo](#undo--redo).       |

---

## Layout

| Term             | Definition                                                                                                                                                                                                  |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Page**         | A single double-sided spread holding one or more [slots](#slot), each spread carrying its own [page mode](#page-mode).                                                                                      |
| **Cover**        | The first page (index 0) of a [project](#project), which may use a different size or layout than the inner pages.                                                                                           |
| **Slot**         | A positioned, sized rectangle on a [page](#page) that holds exactly one [photo](#photo).                                                                                                                    |
| **Aspect Ratio** | The width-to-height ratio of a [photo](#photo) or [slot](#slot).                                                                                                                                            |
| **Page Mode**    | A per-page flag: **Auto** lets fotobuch recalculate the [slot](#slot) arrangement freely; **Manual** locks slot positions so they are not changed during [rebuild](#rebuild).                               |
| **Bleed**        | Extra image area printed beyond the trim line, ensuring no white border appears after cutting.                                                                                                              |
| **Margin**       | White space kept between the content area and the page edge (inside the trim line).                                                                                                                         |
| **DPI**          | Dots per inch — the print resolution of a rendered [page](#page) in the [release build](#release-build).                                                                                                    |

---

## GUI Areas

| Term              | Definition                                                                                                                                                                                             |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Toolbar**       | The horizontal bar at the top of the window containing buttons for all main actions.                                                                                                                   |
| **Photo Pool**    | The panel on the left side of the window listing all imported [photos](#photo) available for [placement](#photo-place).                                                                                |
| **Central Panel** | The main canvas area in the centre of the window where pages are displayed and edited.                                                                                                                 |
| **HUD**           | The overlay controls that appear below a [page](#page) in the [central panel](#central-panel) when the pointer hovers over it — includes a [rebuild](#rebuild) button, mode toggle, and delete button. |
| **New Page Area** | A drop zone shown in the [central panel](#central-panel) between pages where [photos](#photo) can be dropped to create a new [page](#page) at that position.                                           |
| **Nav Bar**       | The panel on the right side of the window showing miniature [thumbnails](#thumbnail) of all pages for quick navigation and [selection](#selection).                                                    |
| **Status Bar**    | The bar at the bottom of the window showing the current operation state and progress messages.                                                                                                         |
| **Thumbnail**     | A small [preview](#preview) image of a [page](#page) or [photo](#photo), shown in the [nav bar](#nav-bar) and [photo pool](#photo-pool).                                                               |
| **Zoom**          | A scaling factor applied to the [central panel](#central-panel) view that changes how large pages appear on screen without affecting the actual layout.                                                |

---

## Actions

| Term               | Definition                                                                                                                                                                                                                                                                                     |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Photo Add**      | Importing one or more image files from disk into the [photo pool](#photo-pool) of the current [project](#project).                                                                                                                                                                             |
| **Photo Place**    | Assigning selected [photos](#photo) to [slots](#slot) on existing [pages](#page) by timestamp (use [rebuild](#rebuild) first to create the pages).                                                                                                                                            |
| **Unplace**        | Removing a [photo](#photo) from its [slot](#slot), returning it to the [photo pool](#photo-pool) as unplaced.                                                                                                                                                                                  |
| **Slot Swap**      | Exchanging the [photos](#photo) in two [slots](#slot); blocked within one [page](#page) when their [aspect ratios](#aspect-ratio) differ.                                                                                                                                                      |
| **Slot Move**      | Relocating a [photo](#photo) from one [slot](#slot) to a slot on a different [page](#page), or to a [Manual-mode](#page-mode) slot at an arbitrary position.                                                                                                                                   |
| **Dragging**       | Moving a [photo](#photo) with the mouse — left-drag from the [photo pool](#photo-pool) to [place](#photo-place), right-drag a [slot](#slot) in the [central panel](#central-panel) to [swap](#slot-swap) or [move](#slot-move).                                                                |
| **Rebuild**        | Recalculating the page layout — targeted (only the selected [pages](#page), keeping [photo](#photo) assignments) or full (redistributing all photos from scratch).                                                                                                                            |
| **Undo / Redo**    | Navigating backwards or forwards through the [history](#history) of a [project](#project), reverting or re-applying changes one step at a time.                                                                                                                                                |
| **Selection**      | A set of [pages](#page) or [photos](#photo) currently marked for the next action (rebuild, place, delete, etc.).                                                                                                                                                                               |
| **Multiselection** | Selecting more than one [page](#page) or [photo](#photo) at once, for example with `Ctrl+Click` or `Shift+Click`.                                                                                                                                                                              |

---

## Output

| Term              | Definition                                                                                                                              |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| **Preview**       | A fast, screen-resolution render of a [page](#page) shown in the [central panel](#central-panel) and [nav bar](#nav-bar) while working. |
| **Release Build** | A full print-quality PDF export of the complete photo book, saved as `{project-name}_final.pdf` in the [project](#project) folder.      |
