# Command Overview

All commands follow the pattern `fotobuch <command> [options]`.
Run `fotobuch --help` or `fotobuch <command> --help` for details,
or see the [Full Flag Reference](reference-generated.md).

> **Your original photos are never modified.** fotobuch only reads your source
> files to create cached copies at the configured DPI. Commands like `remove`
> delete photos from the project YAML — your originals on disk are untouched.

## Commands at a glance

| Command          | What it does                                                   |
| ---------------- | -------------------------------------------------------------- |
| `project new`    | Create a new photobook project                                 |
| `project list`   | List all projects in the current repo                          |
| `project switch` | Switch to another project (checks out its Git branch)          |
| `init`           | Alias for `project new` (shorthand for first-time setup)       |
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

For all flags and exact syntax see the [Full Flag Reference](reference-generated.md).
For available config keys see [Configuration](../general/configuration.md).

### `remove` vs. `unplace`

- **`remove`** deletes photos from the project. They are gone (unless you `undo`).
- **`unplace`** takes photos off their page but keeps them in the project. They
  become unplaced and can be re-placed with `fotobuch place`.

Use `remove --keep-files` if you want remove-like pattern matching but
unplace-like behaviour (photos stay, just lose their page assignment).

Use `remove --unplaced` to delete **all** photos that are not yet assigned to
any page in one step (no pattern argument needed):

```bash
fotobuch remove --unplaced
```

### `build` vs. `rebuild`

- **`build`** renders the PDF and only re-solves pages that changed since the
  last build. On the first run it solves everything.
- **`rebuild --page N`** forces the solver to re-optimize page N from scratch,
  even if nothing changed. Useful when you're not happy with a layout.
- **`rebuild --all`** re-solves every page.
- **`rebuild --range-start A --range-end B`** re-solves pages A through B
  (0-based, inclusive). Add `--flex N` to let the solver vary the page count in
  that range by ±N:

  ```bash
  fotobuch rebuild --range-start 3 --range-end 7 --flex 2
  ```

### `place --into-new-page-at`

`place --into` places photos onto an existing page. `--into-new-page-at <POS>`
creates a **new** page at position POS and places the selected photos there:

```bash
fotobuch place --filter "panorama.*" --into-new-page-at 4
```
