# Full Flag Reference

> This page is auto-generated from the CLI source. Run `cargo run --example generate-cli-docs` to regenerate.

<!-- AUTO-GENERATED: do not edit by hand -->

# Command-Line Help for `fotobuch`

This document contains the help content for the `fotobuch` command-line program.

**Command Overview:**

* [`fotobuch`↴](#fotobuch)
* [`fotobuch add`↴](#fotobuch-add)
* [`fotobuch build`↴](#fotobuch-build)
* [`fotobuch build release`↴](#fotobuch-build-release)
* [`fotobuch rebuild`↴](#fotobuch-rebuild)
* [`fotobuch place`↴](#fotobuch-place)
* [`fotobuch unplace`↴](#fotobuch-unplace)
* [`fotobuch page`↴](#fotobuch-page)
* [`fotobuch page move`↴](#fotobuch-page-move)
* [`fotobuch page split`↴](#fotobuch-page-split)
* [`fotobuch page combine`↴](#fotobuch-page-combine)
* [`fotobuch page swap`↴](#fotobuch-page-swap)
* [`fotobuch page info`↴](#fotobuch-page-info)
* [`fotobuch page weight`↴](#fotobuch-page-weight)
* [`fotobuch remove`↴](#fotobuch-remove)
* [`fotobuch status`↴](#fotobuch-status)
* [`fotobuch config`↴](#fotobuch-config)
* [`fotobuch history`↴](#fotobuch-history)
* [`fotobuch undo`↴](#fotobuch-undo)
* [`fotobuch redo`↴](#fotobuch-redo)
* [`fotobuch project`↴](#fotobuch-project)
* [`fotobuch project new`↴](#fotobuch-project-new)
* [`fotobuch project list`↴](#fotobuch-project-list)
* [`fotobuch project switch`↴](#fotobuch-project-switch)
* [`fotobuch init`↴](#fotobuch-init)
* [`fotobuch completions`↴](#fotobuch-completions)

## `fotobuch`

Photobook layout solver and project manager

**Usage:** `fotobuch <COMMAND>`

###### **Subcommands:**

* `add` — Add photos to the project
* `build` — Calculate layout and generate preview or final PDF
* `rebuild` — Force re-optimization of pages or page ranges
* `place` — Place unplaced photos into the book
* `unplace` — Remove photos from the layout at a page:slot address (they stay in the project)
* `page` — Page manipulation commands (move, split, combine, swap)
* `remove` — Remove photos or groups from the book
* `status` — Show project status
* `config` — Show resolved configuration with defaults
* `history` — Show project change history
* `undo` — Undo the last N commits (default: 1)
* `redo` — Redo N previously undone commits (default: 1)
* `project` — Project management commands
* `init` — Create a new photobook project (alias for `project new`)
* `completions` — Print shell completion script to stdout



## `fotobuch add`

Add photos to the project

**Usage:** `fotobuch add [OPTIONS] [PATHS]...`

###### **Arguments:**

* `<PATHS>` — Directories or files containing photos to add

###### **Options:**

* `--allow-duplicates` — Allow adding duplicate photos (by hash)
* `--filter-xmp <REGEX>` — Only include photos whose XMP metadata matches this regex (can be repeated, all must match)
* `--filter <REGEX>` — Only include photos whose source path matches this regex pattern (can be repeated, all must match)
* `-d`, `--dry` — Preview what would be added without writing anything
* `--update` — Re-add photos whose path already exists but whose content has changed
* `-r`, `--recursive` — Scan directories recursively (each subdir becomes its own group)
* `--weight <WEIGHT>` — Area weight for all imported photos (default: 1.0)

  Default value: `1`



## `fotobuch build`

Calculate layout and generate preview or final PDF

**Usage:** `fotobuch build [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `release` — Generate final high-quality PDF at 300 DPI

###### **Options:**

* `--pages <PAGES>` — Only rebuild specific pages (0-based, comma-separated or repeated flag)



## `fotobuch build release`

Generate final high-quality PDF at 300 DPI

**Usage:** `fotobuch build release [OPTIONS]`

###### **Options:**

* `--force` — Force release even if layout has uncommitted changes



## `fotobuch rebuild`

Force re-optimization of pages or page ranges

**Usage:** `fotobuch rebuild [OPTIONS]`

###### **Options:**

* `--page <PAGE>` — Single page to rebuild (0-based index)
* `--range-start <RANGE_START>` — Start of page range (0-based index, requires --range-end)
* `--range-end <RANGE_END>` — End of page range (0-based index, inclusive, requires --range-start)
* `--flex <FLEX>` — Allow page count to vary by +/- N (only with range)

  Default value: `0`
* `--all` — Rebuild all pages from scratch



## `fotobuch place`

Place unplaced photos into the book

**Usage:** `fotobuch place [OPTIONS]`

###### **Options:**

* `--filter <REGEX>` — Only place photos matching this regex pattern (can be repeated, all must match)
* `--into <INTO>` — Place all matching photos onto this specific page (0-based index)



## `fotobuch unplace`

Remove photos from the layout at a page:slot address (they stay in the project)

The page is deleted automatically if it becomes empty.

**Usage:** `fotobuch unplace <ADDRESS>`

###### **Arguments:**

* `<ADDRESS>` — Slot address: "3:2" (slot 2 on page 3), "3:2,7", "3:2..5", "3:2..5,7"



## `fotobuch page`

Page manipulation commands (move, split, combine, swap)

**Usage:** `fotobuch page <COMMAND>`

###### **Subcommands:**

* `move` — Move or unplace photos between pages
* `split` — Split a page at a slot: photos from that slot onwards move to a new page inserted after
* `combine` — Merge pages onto the first one, then delete the now-empty source pages
* `swap` — Swap photos between two addresses (only single numbers or ranges, no comma lists)
* `info` — Show photo metadata for slots on a page
* `weight` — Set area_weight for one or more slots



## `fotobuch page move`

Move or unplace photos between pages

Two forms: "SRC to DST" (move) and "SRC out" (unplace).

Addressing: 3 = whole page, 3:2 = slot 2 on page 3,
3:1..3,7 = slots 1-3 and 7, 4+ = new page after 4.

Move examples: "3:2 to 5", "3,4 to 5", "3:2 to 4+".
Unplace examples: "3 out", "3:2 out".

See the documentation for the full addressing syntax.

**Usage:** `fotobuch page move [ARGS]...`

###### **Arguments:**

* `<ARGS>` — Expression passed as space-separated tokens, e.g.: 3:2 to 5



## `fotobuch page split`

Split a page at a slot: photos from that slot onwards move to a new page inserted after

Shortcut for `page move PAGE:SLOT.. to PAGE+`. Error if SLOT is the first slot (would leave the original page empty).

**Usage:** `fotobuch page split <ADDRESS>`

###### **Arguments:**

* `<ADDRESS>` — Address "PAGE:SLOT", e.g. "3:4" splits page 3 at slot 4



## `fotobuch page combine`

Merge pages onto the first one, then delete the now-empty source pages

All following page numbers shift down accordingly.

**Usage:** `fotobuch page combine <PAGES>`

###### **Arguments:**

* `<PAGES>` — Pages expression: "3,5" (page 5 onto 3) or "3..5" (pages 4-5 onto 3)



## `fotobuch page swap`

Swap photos between two addresses (only single numbers or ranges, no comma lists)

Page swap: "3 5" swaps pages, "1..2 5..9" swaps blocks.
Slot swap: "3:2 5:6" swaps individual slots,
"3:2..4 5:6..9" swaps slot ranges (different sizes ok).

Errors on overlapping ranges or comma-separated lists as operands.

**Usage:** `fotobuch page swap <LEFT> <RIGHT>`

###### **Arguments:**

* `<LEFT>` — Left address: "3:2", "3:1..3", "3", "3..6"
* `<RIGHT>` — Right address: "5:6", "5:2..4", "5", "8..11"



## `fotobuch page info`

Show photo metadata for slots on a page

Address forms: `3` (all slots), `3:2` (single slot), `3:1..3,7` (slots 1–3 and 7).

Without flags: full table (or vertical view for a single slot). With a flag: machine-readable single-field output.

**Usage:** `fotobuch page info [OPTIONS] <ADDRESS>`

###### **Arguments:**

* `<ADDRESS>` — Address: "3", "3:2", "3:1..3,7"

###### **Options:**

* `--weights` — Output only area weights (format: page:slot=weight)
* `--ids` — Output only photo IDs
* `--pixels` — Output only pixel dimensions



## `fotobuch page weight`

Set area_weight for one or more slots

Examples: `3:2 2.0` (single slot), `3:1..3,7 2.0` (multiple slots), `3 2.0` (whole page).

**Usage:** `fotobuch page weight <ADDRESS> <WEIGHT>`

###### **Arguments:**

* `<ADDRESS>` — Address: "3", "3:2", "3:1..3,7"
* `<WEIGHT>` — Weight value (must be > 0)



## `fotobuch remove`

Remove photos or groups from the book

**Usage:** `fotobuch remove [OPTIONS] [PATTERNS]...`

###### **Arguments:**

* `<PATTERNS>` — Photos, group names, or regex patterns to remove (can be repeated)

###### **Options:**

* `--keep-files` — Only remove from layout, keep photos in the project (makes them unplaced)
* `--unplaced` — Remove all photos that are not placed in any layout page



## `fotobuch status`

Show project status

**Usage:** `fotobuch status [PAGE]`

###### **Arguments:**

* `<PAGE>` — Show detailed information for a specific page (0-based index)



## `fotobuch config`

Show resolved configuration with defaults

**Usage:** `fotobuch config`



## `fotobuch history`

Show project change history

**Usage:** `fotobuch history [OPTIONS]`

###### **Options:**

* `-n <COUNT>` — Number of entries to show (0 = all)

  Default value: `5`



## `fotobuch undo`

Undo the last N commits (default: 1)

**Usage:** `fotobuch undo [STEPS]`

###### **Arguments:**

* `<STEPS>` — Number of steps to undo

  Default value: `1`



## `fotobuch redo`

Redo N previously undone commits (default: 1)

**Usage:** `fotobuch redo [STEPS]`

###### **Arguments:**

* `<STEPS>` — Number of steps to redo

  Default value: `1`



## `fotobuch project`

Project management commands

**Usage:** `fotobuch project <COMMAND>`

###### **Subcommands:**

* `new` — Create a new photobook project
* `list` — List all photobook projects
* `switch` — Switch to another photobook project



## `fotobuch project new`

Create a new photobook project

**Usage:** `fotobuch project new [OPTIONS] --width <WIDTH> --height <HEIGHT> <NAME>`

###### **Arguments:**

* `<NAME>` — Project name

###### **Options:**

* `--width <WIDTH>` — Page width in millimeters
* `--height <HEIGHT>` — Page height in millimeters
* `--bleed <BLEED>` — Bleed margin in millimeters

  Default value: `3`
* `--parent-dir <PARENT_DIR>` — Parent directory where project will be created (default: current directory)
* `--quiet` — Suppress welcome message

  Default value: `false`
* `--with-cover` — Create project with an active cover page

  Default value: `false`
* `--cover-width <COVER_WIDTH>` — Cover width in millimeters (defaults to page_width * 2 if --with-cover is set, with warning)
* `--cover-height <COVER_HEIGHT>` — Cover height in millimeters (defaults to page_height if --with-cover is set, with warning)
* `--spine-grow-per-10-pages-mm <SPINE_GROW_PER_10_PAGES_MM>` — Spine width growth per 10 inner pages in mm (auto mode, conflicts with --spine-mm)
* `--spine-mm <SPINE_MM>` — Fixed spine width in mm (conflicts with --spine-grow-per-10-pages-mm)
* `--margin-mm <MARGIN_MM>` — Inner margin in millimeters (default: 0)

  Default value: `0`



## `fotobuch project list`

List all photobook projects

**Usage:** `fotobuch project list`



## `fotobuch project switch`

Switch to another photobook project

**Usage:** `fotobuch project switch <NAME>`

###### **Arguments:**

* `<NAME>` — Project name to switch to



## `fotobuch init`

Create a new photobook project (alias for `project new`)

**Usage:** `fotobuch init [OPTIONS] --width <WIDTH> --height <HEIGHT> <NAME>`

###### **Arguments:**

* `<NAME>` — Project name

###### **Options:**

* `--width <WIDTH>` — Page width in millimeters
* `--height <HEIGHT>` — Page height in millimeters
* `--bleed <BLEED>` — Bleed margin in millimeters

  Default value: `3`
* `--parent-dir <PARENT_DIR>` — Parent directory where project will be created (default: current directory)
* `--quiet` — Suppress welcome message

  Default value: `false`
* `--with-cover` — Create project with an active cover page

  Default value: `false`
* `--cover-width <COVER_WIDTH>` — Cover width in millimeters
* `--cover-height <COVER_HEIGHT>` — Cover height in millimeters
* `--spine-grow-per-10-pages-mm <SPINE_GROW_PER_10_PAGES_MM>` — Spine width growth per 10 inner pages in mm
* `--spine-mm <SPINE_MM>` — Fixed spine width in mm
* `--margin-mm <MARGIN_MM>` — Inner margin in millimeters (default: 0)

  Default value: `0`



## `fotobuch completions`

Print shell completion script to stdout

Usage:
  fotobuch completions --shell bash   >> ~/.bash_completion
  fotobuch completions --shell zsh    >> ~/.zshrc
  fotobuch completions --shell fish   > ~/.config/fish/completions/fotobuch.fish
  fotobuch completions --shell powershell >> $PROFILE

**Usage:** `fotobuch completions --shell <SHELL>`

###### **Options:**

* `--shell <SHELL>` — Shell to generate completions for

  Possible values: `bash`, `elvish`, `fish`, `powershell`, `zsh`




<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
