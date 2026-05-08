# Glossary

Domain-specific terms used throughout fotobuch. This is the authoritative definition source — all code, documentation, and communication should use these terms consistently.

---

## Data and Storage

### Vault

A folder on disk that contains one or more [projects](#project), stored as a single git repository. fotobuch remembers the last-used vault and re-opens it on startup.

### Project

A single photo book, consisting of one YAML file (photo list, layout, configuration) and a Typst template. Multiple projects are stored together in a [vault](#vault).

### Photo

A single image file imported into a [project](#project). Identified by a stable content hash. Belongs to exactly one [photo group](#photo-group).

### Photo Group

A named collection of [photos](#photo), typically corresponding to a source folder.

### Photo Weight

A numeric value assigned to a [photo](#photo) that influences how much space the layout gives it relative to other photos on the same page.

---

## Layout

### Slot

A positioned, sized rectangle on a page that holds exactly one [photo](#photo).

### Page Mode

A per-page flag: **Auto** lets fotobuch recalculate the [slot](#slot) arrangement freely; **Manual** locks slot positions so they are not changed during rebuild.

### Bleed

Extra image area printed beyond the trim line, ensuring no white border appears after cutting.

### Margin

White space kept between the content area and the page edge (inside the trim line).

### DPI

Dots per inch — the print resolution of a rendered page. Higher DPI means sharper output in the [release build](#release-build).

---

## GUI Areas

### Toolbar

The horizontal bar at the top of the window with buttons for all main actions.

### Photo Pool

The panel on the left side of the window listing all imported [photos](#photo) available for placement.

### Central Panel

The main canvas area in the centre of the window showing the page being worked on.

### HUD

The overlay controls that appear above a page in the [central panel](#central-panel) when the pointer hovers over it — includes a [rebuild](#rebuild) button and page info.

### New Page Area

A drop zone shown in the [central panel](#central-panel) between pages where photos can be dropped to create a new page at that position.

### Nav Strip

The panel on the right side of the window showing miniature thumbnails of all pages for quick navigation.

### Status Bar

The bar at the bottom of the window showing the current state and progress messages.

---

## Actions

### Photo Add

Importing one or more image files into the [photo pool](#photo-pool) of the current [project](#project).

### Photo Place

Assigning selected [photos](#photo) from the [photo pool](#photo-pool) to [slots](#slot) on one or more pages, either automatically (by pressing `P`) or by drag-and-drop.

### Slot Swap

Exchanging the [photos](#photo) in two [slots](#slot). When both slots have the same aspect ratio, only the photos switch; when ratios differ on the same page, the swap is blocked (auto rebuild would undo it).

### Slot Move

Relocating a [photo](#photo) from one [slot](#slot) to a slot on a different page, or to a Manual-mode slot at an arbitrary position.

### Dragging

Moving a [photo](#photo) or page with the mouse. Left-drag photos from the [photo pool](#photo-pool) to place them; right-drag slots in the [central panel](#central-panel) to swap or move them.

### Rebuild

Asking fotobuch to recalculate the layout for one or more pages. A targeted rebuild recalculates only the selected pages (keeping [photo](#photo) assignments unchanged); a full rebuild also redistributes all photos across pages from scratch.

### Selection

A set of pages or [photos](#photo) that are currently marked for the next action (rebuild, place, delete, etc.).

### Multiselection

Selecting more than one page or [photo](#photo) at once, for example with `Ctrl+Click` or `Shift+Click`.

---

## Output

### Preview

A fast, screen-resolution render shown in the [central panel](#central-panel) and [nav strip](#nav-strip) while working.

### Release Build

A full print-quality PDF export of the complete photo book, saved as `{project-name}_final.pdf` in the [project](#project) folder.
