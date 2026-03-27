# Core Concepts

Before diving in, here's how the pieces fit together.

## Projects

A fotobuch **project** is a set of files tracked by Git — a YAML config, a Typst
template, and cached images. When you run `fotobuch project new`, it creates
a directory with a Git repo inside.

Multiple projects can live in the **same Git repo**, each on its own branch
(`fotobuch/<name>`). Switching projects (`fotobuch project switch`) is a
Git checkout under the hood — your working directory swaps to the other
project's state.

## Photos and groups

When you `fotobuch add /some/folder`, all photos in that folder become a
**group**. Groups matter because the solver tries to keep photos from the same
group together on the same page (or on neighbouring pages). Think of a group
as "photos from one occasion".

Each subfolder you add is a separate group. With `--recursive`, every subfolder
becomes its own group automatically.

Groups are sorted chronologically — by the date in the folder name if there is
one, otherwise by the oldest photo's timestamp.

## Placed vs. unplaced

A photo can be in one of two states:

| State        | Meaning                                          |
| ------------ | ------------------------------------------------ |
| **unplaced** | In the project, but not assigned to any page yet |
| **placed**   | Assigned to a specific slot on a specific page   |

When you run `fotobuch build` for the **first time**, all photos are placed
automatically — the solver distributes them across pages.

After that first build, any **newly added** photos start as unplaced. You need
to run `fotobuch place` to put them into the layout before the next
`fotobuch build`.

`fotobuch status` always shows how many photos are unplaced.

## Pages and slots

The solver arranges photos into a grid-like layout on each page. Each photo
occupies a **slot** — a rectangular area with a specific position and size.

Both pages and slots are **numbered from 0**:

```
Page 0          Page 1          Page 2
┌──┬──┬──┐     ┌─────┬──┐     ┌──┬─────┐
│0 │1 │2 │     │  0  │1 │     │0 │     │
├──┴──┼──┤     │     ├──┤     ├──┤  2  │
│  3  │4 │     ├─────┤2 │     │1 │     │
└─────┴──┘     └─────┴──┘     └──┴─────┘
```

Slots are ordered left-to-right, top-to-bottom (reading order). Use
`fotobuch status <page>` to see the slot numbers for a specific page.

A **slot address** identifies one or more photos:

| Address    | Meaning                                                |
| ---------- | ------------------------------------------------------ |
| `3`        | All slots on page 3                                    |
| `3:2`      | Slot 2 on page 3                                       |
| `3:2..5`   | Slots 2 through 5 on page 3                            |
| `3:2..5,7` | Slots 2–5 and slot 7 on page 3                         |
| `4+`       | New page inserted after page 4 (move destination only) |

## Two ways to address photos

fotobuch has two different ways to refer to photos, depending on whether they
are placed in the layout or not:

**By filename or pattern** — used for photos that are not yet placed, or when
you want to work with photos regardless of their layout position. Matches
against the photo's source path or XMP metadata.

| Command                               | What it does                                  |
| ------------------------------------- | --------------------------------------------- |
| `fotobuch add --filter "pattern"`     | Import only matching photos                   |
| `fotobuch add --filter-xmp "pattern"` | Import only photos whose XMP metadata matches |
| `fotobuch place --filter "pattern"`   | Place only matching unplaced photos           |
| `fotobuch remove "pattern"`           | Remove matching photos from the project       |

**By slot address** — used for photos that are already placed on a page. Each
placed photo has a unique address like `3:2` (page 3, slot 2). See
[Pages and slots](#pages-and-slots) below for the full syntax.

| Command                        | What it does                        |
| ------------------------------ | ----------------------------------- |
| `fotobuch page move 3:2 to 5`  | Move a placed photo to another page |
| `fotobuch page swap 3:2 5:1`   | Swap two placed photos              |
| `fotobuch page weight 3:2 2.0` | Change a placed photo's weight      |
| `fotobuch page info 3:2`       | Show metadata for a placed photo    |
| `fotobuch unplace 3:2`         | Remove a placed photo from its slot |

**Rule of thumb:** use filename patterns when working with *unplaced* photos
(`add`, `place`, `remove`), and use slot addresses when working with *placed*
photos (`page move`, `page swap`, `page weight`, `unplace`).

## Weights

Every photo has an **area weight** (default: 1.0). The solver uses weights to
decide how much space each photo gets relative to its neighbours on the same
page.

- Weight 2.0 → roughly twice the area of a weight-1.0 photo
- Weight 0.5 → roughly half

Set weights when adding (`fotobuch add --weight 3`) or per-slot afterward
(`fotobuch page weight 2:0 3.0`).

## Cover

If you create a project with `--with-cover`, page 0 becomes the cover. The
cover spans the full width (front + spine + back) and has its own bleed and
margin settings. See [Known Limitations](known_limitations.md) for current
caveats.

## Build pipeline

```
fotobuch add     →  photos enter the project (unplaced)
fotobuch place   →  unplaced photos get assigned to pages
fotobuch build   →  solver optimizes layout, renders preview PDF
fotobuch build release  →  renders final PDF at 300 DPI
```

The first `build` implicitly places all photos, so you can skip `place`
on a fresh project.

Every command that changes the layout creates a **Git commit** automatically.
Use `fotobuch undo` / `fotobuch redo` to navigate the history, and
`fotobuch history` to see the log.

## The `.fotobuch` cache

fotobuch stores resized preview and final images in a `.fotobuch/` directory
inside your project. This is purely a cache — you can delete it at any time
without losing data. The next `build` will regenerate whatever it needs.
