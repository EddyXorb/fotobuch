# Core Concepts

## Projects and vaults

A fotobuch **project** is a pair of files tracked by Git — a YAML config, a Typst
template. Multiple projects can live in the same Git
repository (called a **vault**), each on its own branch. Switching projects
is a Git checkout under the hood.

See the [Glossary](../glossary.md) for precise definitions of vault, project, and template.

## Photos and groups

When you import a folder, all photos in that folder become a **group**. Groups
matter because fotobuch tries to keep photos from the same group together on
the same page (or on neighbouring pages). Think of a group as "photos from one
occasion".

Each subfolder you import is a separate group. Groups are sorted
chronologically — by the date in the folder name if there is one, otherwise by
the oldest photo's timestamp.

## Placed vs. unplaced

A photo can be in one of two states:

| State        | Meaning                                          |
| ------------ | ------------------------------------------------ |
| **unplaced** | In the project, but not assigned to any page yet |
| **placed**   | Assigned to a specific slot on a specific page   |

When you build or place photos for the first time, all unplaced photos are
distributed automatically across pages. After that first build, newly added
photos start as unplaced and need to be placed before the next build.

## Pages and slots

fotobuch arranges photos into a grid-like layout on each page. Each photo
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

Slots are ordered left-to-right, top-to-bottom (reading order).

## Weights

Every photo has an **area weight** (default: 1.0). fotobuch uses weights to
decide how much space each photo gets relative to its neighbours on the same
page. But beware: **the actual chosen layout can considerably deviate from the ideal weight ratios, because the solver also has to respect aspect ratios and other constraints.** 

- Weight 2.0 → roughly twice the area of a weight-1.0 photo
- Weight 0.5 → roughly half

**Set weights at import time** when you already know some photos should dominate:

```bash
fotobuch add /photos/2024-Italy --weight 2.0   # all photos in this folder get weight 2
```

**Adjust weights per slot after placement:**

```bash
fotobuch page weight 3:2 2.0   # slot 2 on page 3 gets weight 2
```

In the GUI: select a slot and press `W`, or right-click → *Set weight…*

Weights only affect photos on the **same page**. A photo with weight 3.0 gets
more space than its page-neighbours, but it has no influence on other pages.
After changing weights, run `fotobuch rebuild --page N` (or press `R` in the
GUI) to let the solver recalculate the layout for that page.

## Cover

If you create a project with a cover enabled, page 0 becomes the cover. The
cover spans the full width (front + spine + back) and has its own bleed and
margin settings. See [Cover Modes](cover-modes.md) for the layout options.

## The `.fotobuch` cache

fotobuch stores resized preview and final images in a `.fotobuch/cache/` directory
inside your project folder.
**This is purely a cache — delete it at any time without
losing data.**
The next build regenerates whatever it needs.

