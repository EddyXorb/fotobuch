# GUI Quickstart

This walkthrough takes you from zero to a print-ready PDF using the GUI.

---

## Step 1 — Create a project

When you open fotobuch for the first time, a welcome screen appears. Click
**New project**, give it a name (e.g. `Italy-2024`), choose a folder to save
it in, and set the page dimensions in millimetres (e.g. 297 × 210 mm = A4
landscape).

fotobuch creates the project and opens it automatically.

---

## Step 2 — Add photos

Click **Add** in the toolbar (or press `Ctrl+O`). Select a folder — all photos
in that folder are imported into the Photo Pool on the left. You can add more
folders at any time; each folder becomes a separate group.

The Photo Pool shows all your imported photos. Unplaced photos are available
for placement; the count is shown at the top.

---

## Step 3 — Create pages and place photos

Click **Rebuild** in the toolbar (or press `R`) with no pages selected, then
confirm **Rebuild all** in the dialog. fotobuch creates all necessary pages and
distributes your photos across them automatically.

Alternatively, drag photos from the Photo Pool onto the dashed drop zones between
pages in the Central Panel to create pages and place photos manually one by one.

---

## Step 4 — Browse and adjust

Use the **Nav Bar** on the right to scroll through all pages and jump to any
page with a click.

If a page doesn't look right, hover over it in the Central Panel and click
the **↻** button in the HUD to rebuild just that page. fotobuch re-runs the
layout solver for that page and shows the updated result.

**Moving photos between pages:**
- Right-drag a slot in the Central Panel to start a drag.
- Drop onto another slot to swap, or drop onto a page's empty area to move.

**Adjusting photo weight:**
- Select a slot and press `W` to open the weight slider.
- Or right-click a slot and choose **Set weight…**
- Higher weight = more space for that photo on the page.

**Undoing changes:**
- Press `Ctrl+Z` to undo, `Ctrl+Y` to redo.
- Click **History** in the toolbar to browse and jump to any earlier state.

---

## Step 5 — Fine-tune the configuration

Click **Config** in the toolbar to open the project configuration. Key settings:

- **page_target** — how many pages you want
- **search_timeout** — how long the solver runs (increase for large books)
- **gap_mm** — space between photos on a page

After changing settings, rebuild the affected pages (`R`) to apply the new layout.

---

## Step 6 — Export for print

When you're happy with the layout, click **Release** in the toolbar
(or press `Ctrl+Shift+B`).

fotobuch re-renders all images at full 300 DPI and writes
`{project-name}_final.pdf` in your project folder. The file opens automatically
when the build is done.

**Do not upload the regular preview PDF** (`{project-name}.pdf`) to a print
service — it renders at screen resolution only.

See [Printing & Export](../general/printing.md) for Saal Digital-specific details.
