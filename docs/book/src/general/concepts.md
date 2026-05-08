# Core Concepts

## Projects and vaults

A fotobuch **project** is a set of files tracked by Git — a YAML config, a Typst
template, and cached images. Multiple projects can live in the same Git
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
page.

- Weight 2.0 → roughly twice the area of a weight-1.0 photo
- Weight 0.5 → roughly half

Weights can be set when importing or adjusted per-slot afterward — both in the
GUI (W key, or right-click a slot → Set weight…) and in the CLI
(`fotobuch page weight`).

## Cover

If you create a project with a cover enabled, page 0 becomes the cover. The
cover spans the full width (front + spine + back) and has its own bleed and
margin settings. See [Cover Modes](cover-modes.md) for the layout options.

## The `.fotobuch` cache

fotobuch stores resized preview and final images in a `.fotobuch/` directory
inside your project. This is purely a cache — delete it at any time without
losing data. The next build regenerates whatever it needs.
