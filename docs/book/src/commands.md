# Command Overview

All commands follow the pattern `fotobuch <command> [options]`.
Run `fotobuch --help` or `fotobuch <command> --help` for details,
or see the [Full Flag Reference](cli/reference-generated.md).

> **Your original photos are never modified.** fotobuch only reads your source
> files to create cached copies at the configured DPI. Commands like `remove`
> delete photos from the project YAML — your originals on disk are untouched.

## Commands at a glance

| Command          | What it does                                                   |
| ---------------- | -------------------------------------------------------------- |
| `project new`    | Create a new photobook project                                 |
| `project list`   | List all projects in the current repo                          |
| `project switch` | Switch to another project (checks out its Git branch)          |
| `add`            | Import photos or folders into the project                      |
| `remove`         | Delete photos from the project entirely                        |
| `place`          | Assign unplaced photos to pages                                |
| `unplace`        | Remove photos from their page slots (they stay in the project) |
| `build`          | Solve layout and render preview PDF                            |
| `build release`  | Render final PDF at full resolution (300 DPI)                  |
| `rebuild`        | Re-run the solver on specific pages                            |
| `page move`      | Move photos between pages                                      |
| `page swap`      | Swap pages or slots                                            |
| `page split`     | Split a page at a slot                                         |
| `page combine`   | Merge pages together                                           |
| `page info`      | Show photo metadata for slots on a page                        |
| `page weight`    | Set the area weight for one or more slots                      |
| `page mode`      | Toggle a page between auto (solver) and manual placement       |
| `page pos`       | Move or scale slots on a manual-mode page                      |
| `status`         | Show project overview (or single-page detail)                  |
| `config show`    | Print the resolved configuration with all defaults             |
| `config set`     | Set a config value using dot-notation (e.g. `book.dpi 150`)    |
| `history`        | Show the project change log                                    |
| `undo`           | Undo the last N changes                                        |
| `redo`           | Redo N undone changes                                          |

### `page mode` — Auto vs. Manual layout

By default every page is in **auto** mode: the genetic-algorithm solver places
photos automatically. Switch a page to **manual** mode to position slots yourself.

```bash
fotobuch page mode 3 m          # page 3 → manual
fotobuch page mode 3..5 a       # pages 3–5 → auto
```

### `page pos` — Move or scale slots (manual mode only)

Repositions one or more slots on a page that is already in manual mode.

```bash
fotobuch page pos 4:2 --by -20,30       # move slot 2 on page 4: −20 mm x, +30 mm y
fotobuch page pos 4:2 --at 100,50       # set slot 2 origin to (100 mm, 50 mm)
fotobuch page pos 4:2 --scale 1.5       # scale slot 2 by 1.5× (origin stays fixed)
fotobuch page pos 4:2..5 --by -20,30    # move slots 2–5 together
fotobuch page pos 4:2 --at 100,50 --scale 2.0
```

`--by` and `--at` are mutually exclusive; `--scale` can be combined with either.
At least one flag is required. The page must be in manual mode first.

### `config show` / `config set` — View and mutate configuration

```bash
fotobuch config show                        # print resolved config with defaults
fotobuch config set book.dpi 150            # change DPI
fotobuch config set book.gap_mm 3.5
fotobuch config set book.cover.active true
fotobuch config set page_layout_solver.mutation_rate 0.4
```

Key uses dot-notation matching the YAML hierarchy.
The old and new value are printed after each change:

```
book.dpi: 300 → 150
```

See [Configuration](configuration.md) for all available keys.

### `remove` vs. `unplace`

- **`remove`** deletes photos from the project. They are gone (unless you `undo`).
- **`unplace`** takes photos off their page but keeps them in the project. They
  become unplaced and can be re-placed with `fotobuch place`.

Use `remove --keep-files` if you want remove-like pattern matching but
unplace-like behaviour (photos stay, just lose their page assignment).

### `build` vs. `rebuild`

- **`build`** renders the PDF and only re-solves pages that changed since the
  last build. On the first run it solves everything.
- **`rebuild --page N`** forces the solver to re-optimize page N from scratch,
  even if nothing changed. Useful when you're not happy with a layout.
- **`rebuild --all`** re-solves every page.

