# 2. Projektstruktur

## Verzeichnisübersicht

```
fotobuch/
├── Cargo.toml                  # Rust dependencies
├── pyproject.toml              # Python package config (optional, dokumentarisch)
├── uv.lock                     # Python dependency lock file (in Rust eingebacken)
├── README.md
├── .gitignore                  # + .venv/, __pycache__/, target/
│
├── .claude/                    # Planungsdokumente
│   ├── 1_overview.md
│   ├── 2_projektstruktur.md
│   ├── 3_rust-module.md
│   ├── 4_python-package.md
│   ├── 5_environment-setup.md
│   ├── 6_json-schnittstelle.md
│   ├── 7_optimierung.md
│   ├── 8_workflow.md
│   └── 9_roadmap.md
│
├── src/                        # Rust Source Code
│   ├── main.rs                 # CLI entry point
│   ├── models.rs               # Datenstrukturen (Photo, Page, Placement)
│   ├── scanner.rs              # Ordner scannen, EXIF lesen
│   ├── python_env.rs           # Python venv management
│   ├── solver.rs               # Solver-Orchestration (CLI/API/Heuristic)
│   └── typst_export.rs         # Typst-Generierung & PDF-Kompilierung
│
├── python/                     # Python Package
│   ├── pyproject.toml          # Package metadata & dependencies
│   ├── uv.lock                 # Locked dependencies
│   │
│   ├── photosolver/            # Python Package
│   │   ├── __init__.py         # Package exports
│   │   ├── __main__.py         # CLI entry point (python -m photosolver)
│   │   ├── cli.py              # Typer CLI commands
│   │   ├── models.py           # FastAPI/Pydantic models
│   │   ├── api.py              # FastAPI REST API
│   │   ├── solver.py           # OR-Tools CP-SAT Solver
│   │   └── utils.py            # Helper functions
│   │
│   ├── docs/                   # Mathematische Dokumentation
│   │   └── solver_model.typ    # CSP-Modell (Typst)
│   │
│   └── tests/                  # Python Tests
│       ├── test_solver.py
│       ├── test_api.py
│       └── test_data/
│           └── sample_input.json
│
├── test_photos/                # Test-Daten
│   ├── artificial_input_generator.py  # Testdaten-Generator (Typer CLI)
│   ├── 2024-07-15_Urlaub/
│   │   ├── IMG_001.jpg
│   │   └── IMG_002.jpg
│   └── 2024-08-20_Geburtstag/
│       └── IMG_100.jpg
│
├── output/                     # Generierte Ausgaben
│   ├── fotobuch.typ            # Typst Source
│   └── fotobuch.pdf            # Final PDF
│
└── target/                     # Rust Build-Artefakte (gitignored)
    └── release/
        └── photobook-solver    # Binary
```

## Wichtige Dateien

### Root-Level

| Datei | Zweck |
|-------|-------|
| `Cargo.toml` | Rust-Dependencies & Package-Config |
| `pyproject.toml` | Python-Package-Metadata (optional) |
| `uv.lock` | Python-Dependencies (wird ins Binary eingebacken) |
| `README.md` | User-Dokumentation |
| `.gitignore` | Git-Ignore-Rules (inkl. `.venv/`, `target/`) |

### Rust Source (`src/`)

| Datei | Verantwortung |
|-------|---------------|
| `main.rs` | CLI, Workflow-Orchestrierung |
| `models.rs` | Rust-Datenstrukturen |
| `scanner.rs` | Foto-Scanning, EXIF-Auslesen |  
| `python_env.rs` | Python venv Setup & Validation |
| `solver.rs` | Solver-Modi (CLI/API/Heuristic) |
| `typst_export.rs` | .typ-Generierung & PDF-Kompilierung |

### Python Package (`python/photosolver/`)

| Datei | Verantwortung |
|-------|---------------|
| `__init__.py` | Package Exports |
| `__main__.py` | Entry Point für `python -m photosolver` |
| `cli.py` | Typer CLI Commands |
| `models.py` | Pydantic/FastAPI Models |
| `api.py` | FastAPI REST Endpoints |
| `solver.py` | OR-Tools CP-SAT Implementation |
| `utils.py` | Helper Functions |

### Python Dokumentation (`python/docs/`)

| Datei | Zweck |
|-------|-------|
| `solver_model.typ` | Mathematische CSP-Modell-Dokumentation (Typst) |

**Hinweis:** Das Typst-Dokument beschreibt das Constraint Satisfaction Problem formal mit OR-Tools CP-SAT Constraints.

## .gitignore

```gitignore
# Rust
/target
Cargo.lock

# Python
.venv/
__pycache__/
*.pyc
*.pyo
*.egg-info/
.pytest_cache/

# Output
/output/*.typ
/output/*.pdf

# IDE
.vscode/
.idea/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db
```

## Nächstes Dokument

➡️ [3. Rust-Module](3_rust-module.md) - Detaillierte Rust-Modul-Beschreibungen
