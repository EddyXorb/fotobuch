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
- [x] Keep version at `0.1.0` in `Cargo.toml` (signals pre-stable API) — already was 0.1.0
- [x] Add `LICENSE` file (AGPL v3)
- [x] Rewrite `README.md` in English + coverage badge at top (skeleton done; placeholders for user)
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
- [x] `cliff.toml` – git-cliff config for Conventional Commits → CHANGELOG + release notes
- [ ] Generate initial `CHANGELOG.md` via git-cliff (run after merging to main)
- [ ] Move out-of-scope items in `TODO.md` to new `## Out of Scope (post v1.0)` section
- [ ] Ensure `cargo test` passes on the current codebase (green baseline before CI setup)

### Phase 2 – GitHub Actions

- [x] `ci.yml` – `cargo test` + `cargo clippy` + `cargo fmt --check` on push/PR + coverage report
- [x] `release.yml` – build Linux + Windows binaries on tag `v*`, run git-cliff,
      SHA256 checksums, create GitHub Release draft
- [x] Smoke test in `release.yml`: runs `./fotobuch --version` after build
- [x] `pages.yml` – build mdBook and deploy to GitHub Pages on push to `main`;
      runs `generate-cli-docs` before `mdbook build`
- [x] `audit.yml` – `cargo audit` (weekly + on push to main)

### Phase 3 – Documentation (mdBook)

Scope for v0.1.0: a slim, practical guide (3–4 pages) aimed at users with little to no
programming experience. The auto-generated CLI flag reference provides completeness;
the handwritten pages provide clarity and examples.

**Setup**
- [x] Set up mdBook structure in `docs/book/`
- [x] Add `clap-markdown` as dev-dependency; `generate-cli-docs` example generates
      `docs/book/src/cli/reference-generated.md` (tested and working)

**Pages for v0.1.0**

1. [x] **Welcome & Installation** – written (`docs/book/src/installation.md`)
2. [x] **Your First Book (Quickstart)** – written (`docs/book/src/quickstart.md`)
3. [x] **Command Overview** – written (`docs/book/src/commands.md`) with YAML config table
4. [x] **Printing & Known Limitations** – written (`docs/book/src/printing.md`)

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
