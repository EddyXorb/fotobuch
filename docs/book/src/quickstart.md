# Your First Book

This walkthrough takes you from zero to a print-ready PDF.
Prerequisites: `fotobuch` installed, a folder of photos on your machine.

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

## Step 2 — Add photos

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

## Step 3 — Build a preview

```bash
fotobuch build
```

On the first run, fotobuch distributes all photos across pages automatically and
renders a preview PDF at lower DPI (fast). Open `Italy-2024.pdf` to review the
result — or open `Italy-2024.typ` in VS Code with
[Typst Preview](https://marketplace.visualstudio.com/items?itemName=mgt19937.typst-preview)
for a live preview.

---

## Step 4 — Adjust the layout

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

## Step 5 — Adding more photos later

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

## Step 6 — Export for print

When you're happy with the layout:

```bash
fotobuch build release
```

This re-renders all images at 300 DPI and writes `Italy-2024_final.pdf`.
The file is ready to upload to your print service (e.g. Saal Digital).

See [Printing](printing.md) for Saal Digital-specific details.
