# Your First Book

This walkthrough takes you from zero to a print-ready PDF.
Prerequisites: `fotobuch` installed, a folder of photos on your machine.

> **Prefer learning by example?** Check out the complete project in `docs/examples/`
> — it has sample images, YAML config, template, and generated PDFs.

---

## Step 1 — Create a project

```bash
fotobuch project new "Italy-2024" --width 297 --height 210
```

This creates a directory `Italy-2024/` with a Git repo, a YAML config, and a
Typst template. The `--width` and `--height` values are in millimetres
(297 × 210 mm = A4 landscape).

Switch into the project folder:

```bash
cd "Italy-2024"
```

> Project names must start with a letter and can only contain letters, digits,
> or dashes.

---

## Step 2 — Review the configuration

Before adding photos, open `Italy-2024.yaml` in a text editor and check the
most important settings. The file already has sensible defaults, but you should
set the **page count** to match the size of book you want:

```yaml
config:
  book:
    page_width_mm: 297.0
    page_height_mm: 210.0
    bleed_mm: 3.0        # required by most print services
    margin_mm: 0.0       # 0 = edge-to-edge; set to e.g. 10 for a white border
  book_layout_solver:
    page_target: 20      # how many pages you want
    page_max: 24         # upper limit — give the solver some room above the target
```

The **book layout solver** is the algorithm that decides how your photos are
distributed across pages. `page_target` is your desired page count;
`page_max` is the hard upper limit. Setting `page_max` a few pages above
`page_target` gives the solver freedom to use an extra page when that produces
a significantly better layout.

If you plan to use a cover, also set:

```yaml
    cover:
      active: true
      front_back_width_mm: 594.0   # total width of front + back panel
      height_mm: 297.0
      spine_text: "Italy 2024"     # or ~ for no text
```

Other settings you might want to adjust: `gap_mm` (space between photos),
`search_timeout` (solver time limit for large books).

See [Configuration](configuration.md) for a full reference of all settings.

> **Tip:** You can always change these later and re-run `fotobuch build`.
> The solver will redistribute photos according to the new settings.

---

## Step 3 — Add photos

Point fotobuch at one or more folders. Each folder becomes a
[group](concepts.md#photos-and-groups) — photos from the same group are kept
together on pages.

```bash
fotobuch add /photos/2024-07-Italy
fotobuch add /photos/2024-08-Hiking
```

Folders with a date in the name (`2024-07-Italy`, `20240715_Rome`) are sorted
chronologically. Folders without a recognisable date are sorted by the oldest
photo's timestamp.

You can also add single files, add recursively (each subfolder = its own group),
or filter:

```bash
# single file
fotobuch add /photos/2024-07-Italy/DSC_0042.jpg

# recursive — each subfolder becomes a group
fotobuch add --recursive /photos/2024-summer

# only photos matching a filename pattern
fotobuch add /photos/2024-07-Italy --filter "DSC_00.*\.jpg"

# only 3-to-5-star photos, giving them more space
fotobuch add /photos/2024-07-Italy --filter-xmp "Rating.*[3-5]" --weight 5
```

Check what was imported:

```bash
fotobuch status
```

---

## Step 4 — Build a preview

```bash
fotobuch build
```

On the first run, fotobuch distributes all photos across pages automatically and
renders a preview PDF at lower DPI (fast). Open `Italy-2024.pdf` to review the
result — or open `Italy-2024.typ` in VS Code with
[Typst Preview](https://marketplace.visualstudio.com/items?itemName=mgt19937.typst-preview)
for a live preview.

---

## Step 5 — Adjust the layout

You will almost certainly want to tweak a few things.

**Swap two pages:**
```bash
fotobuch page swap 3 7
```

**Move a photo to another page:**
```bash
fotobuch page move 3:2 to 5
```
This moves slot 2 on page 3 to page 5. (Pages and slots count from 0 — use
`fotobuch status 3` to see which slot is which.)

**Give a photo more space** (weight > 1 = relatively larger):
```bash
fotobuch page weight 3:2 2.0
```

**Re-solve a single page** (runs the solver again from scratch for that page):
```bash
fotobuch rebuild --page 6
```

**Undo any change:**
```bash
fotobuch undo
```

Every change is committed to Git automatically — `fotobuch history` shows the
log, and `fotobuch undo N` rolls back N steps.

Rebuild the preview after changes:
```bash
fotobuch build
```

---

## Step 6 — Adding more photos later

After the first build, newly added photos start as **unplaced**. Place them
before building:

```bash
fotobuch add /photos/bonus-shots
fotobuch place
fotobuch build
```

You can also place photos onto a specific page:

```bash
fotobuch place --into 4
```

---

## Step 7 — Export for print

When you're happy with the layout:

```bash
fotobuch build release
```

This re-renders all images at 300 DPI and writes `Italy-2024_final.pdf`.
The file is ready to upload to your print service (e.g. Saal Digital).

See [Printing](printing.md) for Saal Digital-specific details.
