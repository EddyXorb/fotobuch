# Release Plan – fotobuch v1.0.0

Planning document for preparing the first public release.
Work happens on branch `claude/prepare-release-v1-w7cvz`.

---

## Decisions Made

| Topic | Decision |
|---|---|
| Language (README/Docs) | English |
| Release binaries | Linux + Windows (`.zip` with `.exe` for Windows) |
| Documentation site | mdBook on GitHub Pages |
| Python dev tools | Moved to `tests/tools/` ✅ |
| cargo doc in Pages | Skip for now – not needed for v1.0 |
| Distribution | Self-build only for now; no crates.io for first release |
| Windows installer | `.exe` as `.zip` only, no `.msi` |
| cargo audit | Yes – in CI and runnable locally (`cargo install cargo-audit && cargo audit`) |
| Code coverage | Yes – report in CI + badge in README |
| Changelog | **git-cliff** – generated on tag only, grouped by type (`feat`, `fix`, etc.) |
| Release drafts | Yes – git-cliff generates grouped release notes on tag → auto-draft; manually publish when ready |
| TODO.md | Keep as-is; move out-of-scope items to new `## Out of Scope (post v1.0)` section |
| Release trigger | Manual (push tag → CI builds + drafts release → you review + publish) |
| First release version | **`0.1.0`** – signals no stable API guarantee yet; `1.0.0` when CLI/YAML format is stable |
| License | **AGPL v3** + commercial contact in README ("For commercial use, contact [email]") |

---

## Open Decisions

*(none so far)*

---

## Work Items

### Phase 1 – Housekeeping

- [x] Move Python dev tools to `tests/tools/` (`artificial_input_generator.py`, `pyproject.toml`, `uv.lock`)
- [ ] Keep version at `0.1.0` in `Cargo.toml` (signals pre-stable API)
- [ ] Add `LICENSE` file (depends on license decision)
- [ ] Rewrite `README.md` in English (current one is outdated and German) + coverage badge at top
- [ ] Generate initial `CHANGELOG.md` via git-cliff
- [ ] Move out-of-scope items in `TODO.md` to new `## Out of Scope (post v1.0)` section

### Phase 2 – GitHub Actions

- [ ] `ci.yml` – `cargo test` + `cargo clippy` + `cargo fmt --check` on push/PR + coverage report
- [ ] `release.yml` – build Linux + Windows binaries on manual tag `v*`, run git-cliff, create GitHub Release draft
- [ ] `pages.yml` – build mdBook and deploy to GitHub Pages on push to `main`
- [ ] `audit.yml` – `cargo audit` (runs in CI, also usable locally)
- [ ] `cliff.toml` – git-cliff config for Conventional Commits → CHANGELOG + release notes

### Phase 3 – Documentation (mdBook)

- [ ] Set up mdBook structure in `docs/book/`
- [ ] Write: Introduction (philosophy, what makes fotobuch different)
- [ ] Write: Getting Started
  - [ ] Installation (build from source + pre-built binaries)
  - [ ] Quickstart (project new → add → build → release, ~5 min)
  - [ ] VS Code Setup (Typst Preview extension, live layout)
- [ ] Write: Concepts
  - [ ] How a Project Works (git-repo inside project, yaml, .typ, caches)
  - [ ] Groups & Ordering (folder names, timestamps, reading flow)
  - [ ] The Layout Solvers (MIP for pages, GA for slots – non-technical)
  - [ ] Cover (what's different, spine, placement)
- [ ] Write: Example Workflows
  - [ ] Example 1 – Fully automatic: add photos → build → release, no manual steps
  - [ ] Example 2 – With manual refinement: rebuild a page, swap pages, weight a photo,
        undo, add cover; shows how much control is available when needed
- [ ] Add `clap-markdown` as dev-dependency; write small `generate-cli-docs` helper binary
      that dumps the full flag reference as `docs/book/src/cli/reference-generated.md`
- [ ] Add CI step in `pages.yml`: run `generate-cli-docs` before `mdbook build`
- [ ] Write: CLI Reference (one handwritten page per command with examples;
      each page links to its section in the auto-generated flag reference via anchor,
      e.g. `[Full flag reference](../reference.md#fotobuch-add)`)
  - [ ] project (new, list, switch)
  - [ ] add (--filter, --recursive, --weight, --update, --dry)
  - [ ] build (preview vs. release, --pages)
  - [ ] rebuild (--page, --range, --all, --flex)
  - [ ] place / unplace (addressing syntax, examples)
  - [ ] page (move, swap, split, combine, info, weight)
  - [ ] remove (--keep-files, --unplaced)
  - [ ] status
  - [ ] config
  - [ ] history
  - [ ] undo / redo
- [ ] Write: Configuration Reference (all YAML fields with type, default, description)
- [ ] Write: Printing & Export – Saal Digital (bleed, TrimBox, DPI, upload)
- [ ] Write: Known Issues & Limitations
  - [ ] Current limitations (cover workflow, etc.)
  - [ ] Out of scope (what fotobuch deliberately does not do)
- [ ] Write: Technical Background (recycle + expand from README)
  - [ ] Page Layout Solver (GA, O(n) gap, DFS)
  - [ ] Book Layout Solver (MIP, decomposition)

### Phase 4 – UX Review & Polish

General usability review of the CLI before release. The goal is to catch rough edges
that a new user would stumble over.

**Cover handling (concrete known issue)**
- [ ] `fotobuch add` currently distributes photos onto the cover page too on first build –
      cover should be excluded from automatic photo distribution
- [ ] Adding a cover currently requires manual YAML editing to position front/back images
      without colliding with the spine. Proposal: cover gets two pre-defined placeholder
      slots (front, back) that the user simply assigns photos to via `fotobuch place` or
      a dedicated command – no YAML editing needed
- [ ] Investigate whether cover slot boundaries (spine width) can be enforced automatically

**CLI syntax review (concrete known issue)**
- [ ] `fotobuch page move A to B` – the `to` keyword feels unexpected in a CLI context;
      evaluate removing it (e.g. `fotobuch page move A B`) or replacing with `->` consistently
- [ ] General pass: review all subcommand names and argument styles for consistency and
      intuitiveness from a first-time-user perspective

**General UX review**
- [ ] Walk through full workflow as a new user (project new → add → build → rebuild → release)
      and note friction points
- [ ] Check all error messages: are they actionable and clear?
- [ ] Check `fotobuch --help` output: is the command hierarchy obvious?

### Phase 5 – Release

- [ ] Final review of all changes
- [ ] Push tag `v0.1.0` manually → triggers release workflow → git-cliff generates notes → draft created
- [ ] Review and publish the auto-generated release draft
- [ ] Verify GitHub Pages deployment

---

## Notes

- Binary name: `fotobuch`
- Current Cargo.toml version: `0.1.0`
- Existing design docs: `docs/design/` (27 markdown files) – good source material for mdBook
- Python dev tools now in `tests/tools/` – run from that directory with `uv run python artificial_input_generator.py`
- No existing CI/CD, no LICENSE file, no CHANGELOG
- cargo audit runnable locally: `cargo install cargo-audit && cargo audit`
