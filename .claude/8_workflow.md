# 8. Workflow & Usage

## Development Workflow

### Git-Workflow (WICHTIG!)

**Für jedes neue Feature:**

```bash
# 1. Neuen Feature-Branch erstellen
git checkout -b feature/area-weight-constraint
# oder: fix/..., refactor/..., docs/...

# 2. Entwickeln mit guten Commits (Conventional Commits)
git commit -m "feat(solver): add area-weight constraint implementation"
git commit -m "test(solver): add unit tests for area-weight"
git commit -m "docs: update solver documentation"

# 3. Tests schreiben - Coverage > 90% halten!
cargo test

# Python: Type-Check + Linting + Tests
cd python
uv run mypy photosolver/         # Type checking
uv run ruff check photosolver/   # Linting
uv run ruff format photosolver/  # Auto-formattinguv run bandit -r photosolver/    # Security linting
uv run safety check              # Dependency vulnerabilitiesuv run pytest --cov=photosolver --cov-report=term

# 4. Push und Review anfordern
git push origin feature/area-weight-constraint

# 5. ⚠️ VOR MERGE: Benutzer muss Branch abnehmen!
# → Code-Review, Tests prüfen, manuell testen

# 6. Nach Abnahme: Merge
git checkout main
git merge feature/area-weight-constraint
```

**Conventional Commits Format:**
- `feat(scope): description` - Neues Feature
- `fix(scope): description` - Bugfix
- `test(scope): description` - Tests hinzufügen/ändern
- `docs: description` - Dokumentation
- `refactor(scope): description` - Code-Umstrukturierung
- `perf(scope): description` - Performance-Verbesserung

**Test-Coverage:**
- ✅ **JEDES neue Feature braucht Unit-Tests**
- ✅ **Coverage muss > 90% bleiben**
- Python: `pytest --cov=photosolver --cov-report=html`
- Rust: `cargo tarpaulin` oder `cargo-llvm-cov`

---

### Lokale Entwicklung

```bash
# 1. Python-Env setup (einmalig)
cd python/
uv sync

# 2. Linting & Type-Check
uv run ruff check photosolver/
uv run mypy photosolver/

# 3. Python-Solver testen
uv run photosolver solve test_input.json --pretty

# 4. API-Server starten
uv run photosolver serve
# → http://localhost:8000/docs

# 5. Rust entwickeln
cd ..
cargo run -- --input test_photos/ --solver api

# 6. CLI-Modus testen
cargo run -- --input test_photos/ --solver cli

# 7. Mit Debug-Output
cargo run -- --input test_photos/ --output output/ --debug
# → Schreibt JSON, Logs, Typst, PDF in output/
```

---

## User Workflow

### Modus A: CLI (Standard)

```bash
# Prerequisites installieren
# - python3 ≥3.10
# - uv

./photobook-solver --input photos/

# Beim ersten Start:
# ⏳ Creating Python environment...
# ✅ Ready!
# Scanning photos/ ...
# Found 47 photos in 3 groups
# Solving layout...
# Generated 12 pages
# ✅ fotobuch.pdf created
```

### Modus B: REST API (schneller)

Terminal 1:
```bash
cd python/
uv run photosolver serve
# 🚀 Server on http://127.0.0.1:8000
```

Terminal 2:
```bash
export PHOTOSOLVER_API_URL=http://localhost:8000
./photobook-solver --input photos/ --solver api
# Schneller - kein Process-Startup!
```

---

## CLI-Flags

```
photobook-solver [OPTIONS] --input <DIR>

OPTIONS:
  --input <DIR>              Input directory
  --output <PATH>            Output directory or PDF file [default: photobook.pdf]
  --solver <MODE>            cli|api [default: cli]
  --debug                    Write intermediate files (JSON, logs, .typ) to output dir
  
  PAGE LAYOUT:
  --page-width <MM>          [default: 297.0]
  --page-height <MM>         [default: 210.0]
  --margin <MM>              [default: 10.0]
  --gap <MM>                 [default: 3.0]
  --max-photos <N>           [default: 4]
  --target-pages <N>         Soft constraint
  
  PHOTO SETTINGS:
  --default-area-weight <F>  [default: 1.0]
  --max-aspect-deviation <F> Max. cropping [default: 0.2]
  
  SOLVER SETTINGS:
  --timeout <SEC>            Solver timeout [default: 30]
  
  OBJECTIVE WEIGHTS (alle anpassbar):
  --weight-aspect <F>        [default: 1.0]
  --weight-area <F>          Area-weight Bestrafung [default: 10.0]
  --weight-groups <F>        Gruppen-Kohäsion [default: 2.0]
  --weight-pages <F>         Seitenzahl-Ziel [default: 0.5]

ENVIRONMENT:
  PHOTOSOLVER_API_URL        For --solver api mode

NOTES:
  • Timeout: mindestens bis zur ersten Lösung, dann beste nehmen
  • API-Mode: Server muss bereits laufen (nicht auto-start)
  • >200 Fotos: evtl. iteratives Verfahren (2 Seiten gleichzeitig)
  • Cropping bevorzugt (kein Letterboxing = schwarze Balken)
  
DEBUG OUTPUT (--debug):
  In output-Ordner werden geschrieben:
  • <timestamp>_<session-id>_input.json     - Solver-Input
  • <timestamp>_<session-id>_output.json    - Solver-Output
  • <timestamp>_<session-id>_rust.log       - Rust-Logs
  • <timestamp>_<session-id>_python.log     - Python-Logs
  • <timestamp>_<session-id>.typ            - Typst-Source
  • <timestamp>_<session-id>.pdf            - Final PDF
  
  Beispiel: 20260303_143022_a8b3f9_input.json
            └─ Zeitstempel  └─ Session-ID (6 Zeichen)
```

---

## Build & Distribution

### Build

```bash
# 1. Lock Python deps
cd python/
uv lock  # → uv.lock

# 2. Build Rust (embeds uv.lock)
cd ..
cargo build --release
# → target/release/photobook-solver (~5-8 MB)
```

### Distribution

**User bekommt:**
- Binary: `photobook-solver` / `.exe`
- Größe: ~5-8 MB
- Enthält: eingebackenes `uv.lock`

**User braucht:**
- Python 3.10+
- uv
- Internet (für erste Installation)

**Erste Verwendung:**
```bash
./photobook-solver --input photos/
# ⏳ Setting up Python (~30 sec einmalig)
# ✅ Ready!
```

### GitHub Release

```bash
# Multi-platform builds
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-pc-windows-msvc
cargo build --release --target aarch64-apple-darwin
```

---

➡️ [9. Roadmap](9_roadmap.md)
