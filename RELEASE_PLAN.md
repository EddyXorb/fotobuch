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
| Distribution | Only self-build for now; crates.io easy to add later (needs license first) |
| Windows installer | `.exe` as `.zip` only, no `.msi` for v1.0 |
| cargo audit | Yes – in CI and runnable locally (`cargo install cargo-audit && cargo audit`) |
| Code coverage | Yes – report in CI + badge in README |
| Release drafts | Yes – auto-draft via `release-drafter` action; manually publish when ready |
| TODO.md | Keep as-is; move out-of-scope items to new `## Out of Scope (post v1.0)` section |
| Release trigger | Manual (you push the tag / click publish) |
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
- [ ] Bump version `Cargo.toml`: `0.1.0` → `1.0.0`
- [ ] Add `LICENSE` file (depends on license decision)
- [ ] Rewrite `README.md` in English (current one is outdated and German) + coverage badge at top
- [ ] Create `CHANGELOG.md`
- [ ] Move out-of-scope items in `TODO.md` to new `## Out of Scope (post v1.0)` section

### Phase 2 – GitHub Actions

- [ ] `ci.yml` – `cargo test` + `cargo clippy` + `cargo fmt --check` on push/PR + coverage report
- [ ] `release.yml` – build Linux + Windows binaries on manual tag `v*`, create GitHub Release
- [ ] `pages.yml` – build mdBook and deploy to GitHub Pages on push to `main`
- [ ] `audit.yml` – `cargo audit` (runs in CI, also usable locally)
- [ ] `release-drafter.yml` – auto-generate release draft from commits

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
- [ ] (Optional) Publish to crates.io
- [ ] Final review of all changes
- [ ] Push tag `v1.0.0` manually → triggers release workflow
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
