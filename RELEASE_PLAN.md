# Release Plan – fotobuch v1.0.0

Planning document for preparing the first public release.
Work happens on branch `claude/prepare-release-v1-w7cvz`.

---

## Decisions Made

| Topic | Decision |
|---|---|
| Language (README/Docs) | English |
| Release binaries | Linux + Windows |
| Documentation site | mdBook on GitHub Pages |
| Python dev tools | Move to `tests/tools/` |
| License | **TBD** – see below |

---

## Open Decisions

### License
Options discussed:
- **MIT** – simplest, most permissive, standard in Rust ecosystem
- **MIT OR Apache-2.0** – Rust standard, adds patent protection
- **GPL v3** – copyleft, requires derivatives to stay open

Key question: Should commercial services be allowed to embed `fotobuch` without giving back?
→ **Decision needed**

### Out-of-scope items (TODO.md)
Pending TODO items that will NOT be in v1.0.0 – move to GitHub Issues or keep in TODO.md?
→ **Decision needed**

### crates.io
Publish to crates.io (`cargo install fotobuch`) in addition to GitHub Releases?
→ **Decision needed**

### Windows installer
Pre-built `.exe` in ZIP, or also an `.msi` installer?
→ **Decision needed**

### cargo audit
Run dependency security audit in CI?
→ **Decision needed**

### Code coverage
Report coverage (e.g. via codecov.io) in CI?
→ **Decision needed**

---

## Work Items

### Phase 1 – Housekeeping

- [ ] Bump version `Cargo.toml`: `0.1.0` → `1.0.0`
- [ ] Add `LICENSE` file (depends on license decision)
- [ ] Move Python dev tools to `tests/tools/` (generator + `pyproject.toml` + `uv.lock`)
- [ ] Rewrite `README.md` in English (current one is outdated and German)
- [ ] Create `CHANGELOG.md`
- [ ] Clean up `TODO.md` (mark v1.0 scope clearly)

### Phase 2 – GitHub Actions

- [ ] `ci.yml` – `cargo test` + `cargo clippy` + `cargo fmt --check` on push/PR
- [ ] `release.yml` – build Linux + Windows binaries on tag `v*`, create GitHub Release
- [ ] `pages.yml` – build mdBook and deploy to GitHub Pages on push to `main`
- [ ] (Optional) `audit.yml` – `cargo audit` for dependency security

### Phase 3 – Documentation (mdBook)

- [ ] Set up mdBook structure in `docs/book/`
- [ ] Write: Introduction & Concepts
- [ ] Write: Installation
- [ ] Write: Quickstart (full workflow A→Z)
- [ ] Write: CLI Reference (all 15+ subcommands)
- [ ] Write: Configuration (YAML schema)
- [ ] Write: Saal Digital / Print settings
- [ ] (Optional) Internals – recycle the 27 existing design docs

### Phase 4 – Release

- [ ] Final review of all changes
- [ ] Tag `v1.0.0`
- [ ] Verify GitHub Release with binaries
- [ ] Verify GitHub Pages deployment

---

## Notes

- Binary name: `fotobuch`
- Current Cargo.toml version: `0.1.0`
- Existing design docs: `docs/design/` (27 markdown files) – good source material for mdBook
- Python component: `pyproject.toml` uses Pillow + pypdf + typer for generating artificial test images
- No existing CI/CD, no LICENSE file, no CHANGELOG
