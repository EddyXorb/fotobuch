# Your First Book

This walkthrough takes you from zero to a print-ready PDF. It assumes you have
`fotobuch` installed and a folder of photos somewhere on your machine.

The whole process takes about 10 minutes the first time.

---

## Step 1 — Create a project

A fotobuch project is a regular folder containing a `fotobuch.yaml` config file
and a Git repository that tracks every change you make.

```bash
fotobuch project new "Italy 2024" --width 297 --height 210
```

This creates a folder called `Italy 2024` in the current directory and
initialises a Git repo inside it. The `--width` and `--height` values are in
millimetres (297 × 210 mm = A4 landscape).

Switch into the project folder:

```bash
cd "Italy 2024"
```

> **Tip:** Run `fotobuch config` at any time to see the full resolved
> configuration with all default values.

---

## Step 2 — Add photos

Point fotobuch at one or more folders. Each folder becomes a *group* — photos
from the same group are kept together on pages.

```bash
fotobuch add /photos/2024-07-Italy
fotobuch add /photos/2024-08-Hiking
```

Folder names that contain a date (`2024-07-Italy`, `20240715_Rome`) are sorted
chronologically. Folders without a date go at the end.

You can also add a single file, or add recursively (each subfolder becomes its
own group):

```bash
fotobuch add /photos/2024-07-Italy/DSC_0042.jpg
fotobuch add --recursive /photos/2024-summer
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

fotobuch solves the layout and renders a preview PDF at 150 DPI (fast).
Open `preview.pdf` (or the `.typ` file in VS Code with Typst Preview) to
review the result.

---

## Step 4 — Adjust the layout

You will almost certainly want to tweak a few things.

**Swap two pages:**
```bash
fotobuch page swap 3 7
```

**Move a photo from one page to another:**
```bash
fotobuch page move 3:2 to 5
```
`3:2` means slot 2 on page 3 (slots are numbered from 1, left-to-right,
top-to-bottom).

**Give a photo more space** (weight > 1 makes it relatively larger):
```bash
fotobuch page weight 3:2 2.0
```

**Force the solver to redo a single page:**
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

## Step 5 — Export for print

When you are happy with the layout:

```bash
fotobuch build release
```

This re-renders all images at 300 DPI and writes `release.pdf`.
The file is ready to upload directly to your print service (e.g. Saal Digital).

See [Printing & Known Limitations](printing.md) for Saal Digital-specific details.
