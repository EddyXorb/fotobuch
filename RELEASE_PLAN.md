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
| License | **TBD** – discuss after all other points are settled |

---

## Open Decisions

### License
Options discussed:
- **MIT** – simplest, most permissive, standard in Rust ecosystem
- **MIT OR Apache-2.0** – Rust standard, adds patent protection
- **GPL v3** – copyleft, requires derivatives to stay open

Key question: Should commercial services be allowed to embed `fotobuch` without giving back?
→ **Decision needed** (to be discussed last)

Note: License is also required before publishing to crates.io.

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
- [ ] Write: Introduction & Concepts
- [ ] Write: Installation
- [ ] Write: Quickstart (full workflow A→Z)
- [ ] Write: CLI Reference (all 15+ subcommands)
- [ ] Write: Configuration (YAML schema)
- [ ] Write: Saal Digital / Print settings
- [ ] (Optional) Internals – recycle the 27 existing design docs in `docs/design/`

### Phase 4 – Release

- [ ] Decide and set license
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
