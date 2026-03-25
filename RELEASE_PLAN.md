# Release Plan – fotobuch v0.1.0

Planning document for preparing the first public release.
Work happens on branch `claude/prepare-release-v1-w7cvz`.

---

## Decisions Made

| Topic | Decision |
|---|---|
| Language (README/Docs) | English |
| Release binaries | Linux + Windows (`.zip` with `.exe` for Windows) |
| Documentation site | mdBook on GitHub Pages (slim: 3–4 pages for v0.1.0) |
| Python dev tools | Moved to `tests/tools/` ✅ |
| cargo doc in Pages | Skip for now – not needed for v0.1.0 |
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
| Branch strategy | Merge `claude/prepare-release-v1-w7cvz` → `main` → tag `v0.1.0` on `main` → CI builds release |
| Binary checksums | SHA256 checksums generated in `release.yml` and attached as release asset |

---

## Open Decisions

*(none so far)*

---

## Work Items

### Phase 1 – Housekeeping

- [x] Move Python dev tools to `tests/tools/` (`artificial_input_generator.py`, `pyproject.toml`, `uv.lock`)
- [ ] Keep version at `0.1.0` in `Cargo.toml` (signals pre-stable API)
- [ ] Add `LICENSE` file (AGPL v3)
- [ ] Rewrite `README.md` in English (current one is outdated and German) + coverage badge at top
- [ ] Clean up README placeholders: replace `YOUREMAIL@example.com` with real address,
      remove `<!-- USER: ... -->` comments, replace or remove `docs/assets/example_spread.jpg`
- [ ] Add `fotobuch init` as alias for `project new` (familiar pattern from git/cargo/npm)
- [ ] Add `fotobuch completions --shell <bash|zsh|fish|powershell>` subcommand
      (clap has built-in support via `clap_complete`, minimal code needed)
- [ ] Create example project in `examples/demo-project/` with 5–10 small public-domain photos
      (e.g. from Unsplash, downscaled to ~100 KB each) and a ready-to-use `fotobuch.yaml`.
      A new user can `cd examples/demo-project && fotobuch build` to see a result immediately.
- [ ] Record terminal demo with [VHS](https://github.com/charmbracelet/vhs) using the example
      project: `init` → `add` → `build` → `page swap` → `rebuild` → `build release` (~30 sec).
      Embed the resulting GIF in the README as hero image (replaces placeholder screenshot).
- [ ] `cliff.toml` – git-cliff config for Conventional Commits → CHANGELOG + release notes
      (needed before CHANGELOG generation)
- [ ] Generate initial `CHANGELOG.md` via git-cliff
- [ ] Move out-of-scope items in `TODO.md` to new `## Out of Scope (post v1.0)` section
- [ ] Ensure `cargo test` passes on the current codebase (green baseline before CI setup)

### Phase 2 – GitHub Actions

- [ ] `ci.yml` – `cargo test` + `cargo clippy` + `cargo fmt --check` on push/PR + coverage report
- [ ] `release.yml` – build Linux + Windows binaries on manual tag `v*`, run git-cliff,
      generate SHA256 checksums, create GitHub Release draft with checksums as asset
- [ ] Add smoke test in `release.yml`: run `./fotobuch --version` after build to verify binary works
- [ ] `pages.yml` – build mdBook and deploy to GitHub Pages on push to `main`;
      includes CI step to run `generate-cli-docs` before `mdbook build`
- [ ] `audit.yml` – `cargo audit` (runs in CI, also usable locally)

### Phase 3 – Documentation (mdBook)

Scope for v0.1.0: a slim, practical guide (3–4 pages) aimed at users with little to no
programming experience. The auto-generated CLI flag reference provides completeness;
the handwritten pages provide clarity and examples.

**Setup**
- [ ] Set up mdBook structure in `docs/book/`
- [ ] Add `clap-markdown` as dev-dependency; write small `generate-cli-docs` helper binary
      that dumps the full flag reference as `docs/book/src/cli/reference-generated.md`

**Pages for v0.1.0**

1. **Welcome & Installation** – What fotobuch is (2–3 sentences), how to install
   (pre-built binary download + build from source), VS Code + Typst Preview setup
2. **Your First Book (Quickstart)** – Step-by-step walkthrough:
   `project new` → `add` → `build` → review → `rebuild` → `build release`.
   Written as a single narrative a non-programmer can follow in ~10 minutes.
   Includes: how groups/folders work, how to weight a photo, how to swap pages.
3. **Command Overview** – One table with all commands, one-line description each,
   link to the matching section in the auto-generated flag reference
   (e.g. `[Full flags](cli/reference-generated.md#fotobuch-add)`).
   Below the table: a short section on the YAML config (key fields, where to find it,
   `fotobuch config` to inspect).
4. **Printing & Known Limitations** – How to export for Saal Digital (bleed, DPI, upload).
   Known limitations (cover workflow, etc.). What fotobuch deliberately does not do.

**Deferred to post-v0.1.0**
- Detailed per-command CLI reference pages with extended examples
- Concepts deep-dive (solvers, project internals, caching)
- Full YAML configuration reference
- Extended workflow examples (fully automatic vs. manual refinement)
- Technical background (GA, MIP, DFS – recycle from README)

### Phase 4 – Pre-Release Review (read-only)

Light review of the CLI experience. **No feature work or breaking changes in this phase** –
only document findings. Code changes go into post-v0.1.0 issues.

- [ ] Walk through full workflow as a new user (`project new` → `add` → `build` → `rebuild` → `build release`)
      and note friction points
- [ ] Check all error messages: are they actionable and clear?
- [ ] Check `fotobuch --help` output: is the command hierarchy obvious?
- [ ] File issues for anything that needs fixing (do not block release)

**Known issues to track as post-v0.1.0 issues (not release blockers):**
- Cover handling: `fotobuch add` distributes photos onto the cover; cover should be excluded
  from automatic distribution. Cover placement requires manual YAML editing – should get
  dedicated slots (front/back) usable via `fotobuch place`.
- CLI syntax: `fotobuch page move A to B` – evaluate removing `to` or replacing with `->`.
- General CLI consistency pass from a first-time-user perspective.

### Phase 5 – Release

- [ ] Final review of all changes
- [ ] Merge branch into `main`
- [ ] Push tag `v0.1.0` on `main` → triggers release workflow → git-cliff generates notes → draft created
- [ ] Review and publish the auto-generated release draft
- [ ] Verify GitHub Pages deployment
- [ ] Verify SHA256 checksums are attached to the release

---

## Notes

- Binary name: `fotobuch`
- Current Cargo.toml version: `0.1.0`
- Existing design docs: `docs/design/` (27 markdown files) – good source material for future mdBook expansion
- Python dev tools now in `tests/tools/` – run from that directory with `uv run python artificial_input_generator.py`
- No existing CI/CD, no LICENSE file, no CHANGELOG
- cargo audit runnable locally: `cargo install cargo-audit && cargo audit`
