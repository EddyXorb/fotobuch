# Command Overview

All commands follow the pattern `fotobuch <command> [options]`.
Run `fotobuch --help` or `fotobuch <command> --help` for details,
or see the [Full Flag Reference](cli/reference-generated.md).

> **Your original photos are never modified.** fotobuch only reads your source
> files to create cached copies at the configured DPI. Commands like `remove`
> delete photos from the project YAML — your originals on disk are untouched.

## Commands at a glance

| Command | What it does |
|---|---|
| `project new` | Create a new photobook project |
| `project list` | List all projects in the current repo |
| `project switch` | Switch to another project (checks out its Git branch) |
| `add` | Import photos or folders into the project |
| `remove` | Delete photos from the project entirely |
| `place` | Assign unplaced photos to pages |
| `unplace` | Remove photos from their page slots (they stay in the project) |
| `build` | Solve layout and render preview PDF |
| `build release` | Render final PDF at full resolution (300 DPI) |
| `rebuild` | Re-run the solver on specific pages |
| `page move` | Move photos between pages |
| `page swap` | Swap pages or slots |
| `page split` | Split a page at a slot |
| `page combine` | Merge pages together |
| `page info` | Show photo metadata for slots on a page |
| `page weight` | Set the area weight for one or more slots |
| `status` | Show project overview (or single-page detail) |
| `config` | Print the resolved configuration with all defaults |
| `history` | Show the project change log |
| `undo` | Undo the last N changes |
| `redo` | Redo N undone changes |

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

