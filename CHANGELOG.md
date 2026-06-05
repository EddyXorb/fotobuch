# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-06-06

The big one: **fotobuch now has a graphical app.** Drag a folder of photos onto
the window, watch the solver lay out your pages live, and refine everything by
dragging — no command line required. The CLI is still here, fully featured, and
operates on the exact same project files.

### Added

- **Graphical app (`fotobuch-gui`).** A visual, interactive interface for building
  and refining photo books:
  - Drag whole folders onto the window to import them as photo groups.
  - Live page rendering — the central panel shows the real Typst output, not a
    mock-up.
  - Drag slots to swap or move photos within and across pages; drop them back into
    the pool to unplace them.
  - Single, range, and multi-selection of slots, with cross-page operations.
  - Pool panel, page navigation bar, and a toolbar covering project switching,
    add, place, rebuild, release, undo/redo & history, config, and drag mode.
  - In-app help window with a lens mode and golden-glow highlighting.
  - Manual page mode: lock a page's slot arrangement so rebuilds leave it untouched.
  - Keyboard shortcuts for all common actions, including undo/redo.
- **Pre-built GUI binary in releases.** Release archives for Linux and Windows now
  ship `fotobuch-gui` alongside the `fotobuch` CLI.
- **CLI: page mode handling** and protection so manual-mode pages are not
  re-solved on rebuild.
- **CLI: `config set`** to change configuration values directly.
- **Move and swap slots onto manual-mode pages.**
- **`preview.write_pdf` option** to skip PDF generation during a build (faster
  iteration; preview images are still produced).
- **`show_preview_watermark` config** option.
- **User-defined base config** support in new projects.

### Changed

- Library API now returns the changed state from commands, and exposes
  `render_pages()` returning RGBA pixmaps — the foundation the GUI and Typst
  preview share.
- Background rendering uses a persistent Typst world for faster rebuilds.
- Release builds disable debug symbols and use `lto=off` for much faster dev
  release builds.

### Fixed

- Skipping PDF generation no longer also skips preview-image generation (the Typst
  preview and the GUI need those).
- Welcome and new-project dialogs now close correctly when their button is clicked.

### Documentation

- Full GUI documentation section in the book (quickstart, overview, panels,
  toolbar, keyboard shortcuts).
- New advanced page: *How the Layout Engine Works*, covering the two-stage solver
  and fotobuch's O(N) exact-gap and DFS reading-order contributions.
- README reworked to put the GUI first, with the personal background moved to its
  own page.

## [0.1.0] - 2026-03-29

Initial release: the CLI photo-book layout engine with automatic page assignment
(MIP) and single-page layout (slicing-tree genetic algorithm), Typst-based PDF
output, git-tracked project history, and DPI-accurate caching.

[0.2.0]: https://github.com/EddyXorb/fotobuch/releases/tag/v0.2.0
[0.1.0]: https://github.com/EddyXorb/fotobuch/releases/tag/v0.1.0
