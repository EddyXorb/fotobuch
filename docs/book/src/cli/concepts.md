# CLI Concepts

## Slot addresses

A **slot address** identifies one or more placed photos on a specific page:

| Address    | Meaning                                                |
| ---------- | ------------------------------------------------------ |
| `3`        | All slots on page 3                                    |
| `3:2`      | Slot 2 on page 3                                       |
| `3:2..5`   | Slots 2 through 5 on page 3                            |
| `3:2..5,7` | Slots 2–5 and slot 7 on page 3                         |
| `4+`       | New page inserted after page 4 (move destination only) |

Use `fotobuch status <page>` to see which slot numbers are on a given page.

## Two ways to address photos

| Method           | Use case                                         | Examples                         |
| ---------------- | ------------------------------------------------ | -------------------------------- |
| **Filename / pattern** | Photos not yet placed, or matching by source path | `add`, `remove`, `place --filter` |
| **Slot address** | Photos already placed on a page                  | `page move`, `page swap`, `page weight` |

**Rule of thumb:** use filename patterns for *unplaced* photos (`add`, `place`,
`remove`); use slot addresses for *placed* photos (`page move`, `page swap`,
`page weight`, `unplace`).

## Build pipeline

```
fotobuch add     →  photos enter the project (unplaced)
fotobuch place   →  unplaced photos get assigned to pages
fotobuch build   →  solver optimises layout, renders preview PDF
fotobuch build release  →  renders final PDF at print resolution
```

The first `build` implicitly places all photos, so you can skip `place` on a
fresh project.

Every command that changes the layout creates a **Git commit** automatically.
Use `fotobuch undo` / `fotobuch redo` to navigate history, and
`fotobuch history` to see the log.

## XMP metadata filtering

`fotobuch add` supports `--filter-xmp <REGEX>` to import only photos whose XMP
metadata matches a pattern. The regex is matched against the **raw XMP XML**
embedded in the image file.

Common use cases:

```bash
# Only photos with 3–5 star rating
fotobuch add /photos --filter-xmp "xmp:Rating>[3-5]<"

# Only photos tagged with a specific label
fotobuch add /photos --filter-xmp "xmp:Label>Vacation<"

# Multiple filters (all must match — AND logic)
fotobuch add /photos --filter-xmp "xmp:Rating>[4-5]<" --filter-xmp "dc:subject>Italy<"
```

Photos without any XMP data are **excluded** when `--filter-xmp` is used.
Use `--filter` instead if you want to match by filename or source path.

> **Tip:** Use `exiftool -xmp -b <file.jpg>` to inspect the raw XMP of a photo
> and find the exact tag names and value format your camera writes.
