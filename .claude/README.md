# Photobook Solver - Planungsdokumentation

Vollständige Planung für OR-Tools Integration im Fotobuch-Projekt.

## 📚 Dokumentation

1. **[Architektur-Übersicht](1_overview.md)** - Komponenten & Datenfluss
2. **[Projektstruktur](2_projektstruktur.md)** - Verzeichnisstruktur
3. **[Rust-Module](3_rust-module.md)** - Detaillierte Rust-Implementierung
4. **[Python-Package](4_python-package.md)** - OR-Tools Solver & API
5. **[Environment-Setup](5_environment-setup.md)** - Dependencies & uv-Integration
6. **[JSON-Schnittstelle](6_json-schnittstelle.md)** - Datenformate & Validierung
7. **[Optimierung](7_optimierung.md)** - Constraints & Objective-Function
8. **[Workflow](8_workflow.md)** - Git, Testing, CLI, Logging
9. **[Roadmap](9_roadmap.md)** - Implementierungs-Phasen & offene Fragen

---

## 🎯 Quick Start (für Entwickler)

### Setup

```bash
# Python-Environment
cd python/
uv sync

# Tests laufen lassen
uv run pytest --cov=photosolver --cov-report=html

# Rust
cd ..
cargo build
cargo test
```

### Development Workflow

```bash
# 1. Feature-Branch erstellen
git checkout -b feature/my-feature

# 2. Entwickeln mit Type Safety + Tests
# ... code ...

# Python: Type-Check + Linting + Tests
cd python
uv run mypy photosolver/         # Type checking (strict)
uv run ruff check photosolver/   # Linting
uv run ruff format photosolver/  # Formatting
uv run pytest --cov

# Rust: Tests + Linting
cd ..
cargo clippy -- -D warnings
cargo fmt --check
cargo test

# 3. Commits (Conventional Commits)
git commit -m "feat(solver): add new constraint"
git commit -m "test(solver): add tests for new constraint"

# 4. Push & Review
git push origin feature/my-feature
# → User-Abnahme vor Merge!
```

### Debug-Mode

```bash
# Rust CLI mit Debug-Output
cargo run -- --input test_photos/ --output output/ --debug

# Erzeugt in output/:
# - 20260303_143022_a8b3f9_input.json
# - 20260303_143022_a8b3f9_output.json
# - 20260303_143022_a8b3f9_rust.log
# - 20260303_143022_a8b3f9_python.log
# - 20260303_143022_a8b3f9.typ
# - 20260303_143022_a8b3f9.pdf
```

---

## 🔑 Wichtige Entscheidungen

### Architektur
- **Rust** - CLI, I/O, Orchestrierung
- **Python** - OR-Tools CP-SAT Solver
- **FastAPI** - REST-API (schneller als subprocess)
- **Typst** - PDF-Generierung

### Features
- ✅ **area_weight** - Relative Fläche pro Foto (default: 1.0)
- ✅ **Chronologische Ordnung** - Hard-Constraint
- ✅ **Gruppen-Kohäsion** - Soft-Constraint
- ✅ **Aspect-Ratio** - Max. 20% Cropping (anpassbar)
- ✅ **Iterativ** - Falls >200 Fotos, 2 Seiten gleichzeitig

### Testing
- 🎯 **Coverage > 90%** für alle Features
- 🧪 **Unit-Tests** bei jedem Feature
- 📸 **Mock-Fotos** für reproduzierbare Tests

### Git-Workflow
- 🌿 **Feature-Branches** - Nie direkt auf main
- 📝 **Conventional Commits** - feat/fix/test/docs/refactor
- 👁️ **User-Review** - Abnahme VOR jedem Merge

### Logging & Debug
- 🔍 **Session-ID** - Gleiche ID für Rust + Python Logs
- 📁 **Output-Ordner** - Alle Dateien an einem Ort
- 🐛 **--debug Flag** - JSON + Logs + Typst schreiben

---

## 📊 Status

| Phase | Status |
|-------|--------|
| Planung | ✅ Abgeschlossen |
| Python Setup | ⏳ TODO |
| Rust Integration | ⏳ TODO |
| OR-Tools Solver | ⏳ TODO |
| Testing | ⏳ TODO |
| Distribution | ⏳ TODO |

---

## 🛠️ Tools

- **Rust**: cargo, clippy, rustfmt, cargo-tarpaulin
- **Python**: uv, pytest, pytest-cov, **ruff** (linter+formatter), **mypy** (type checker)
- **OR-Tools**: CP-SAT Solver (≥9.15)
- **Typst**: Documentation & PDF generation
- **Git**: Conventional Commits, Feature-Branches

Siehe [8_workflow.md](8_workflow.md) für Details.
