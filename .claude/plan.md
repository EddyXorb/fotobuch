# Photobook Solver: OR-Tools Integration Plan

## Projektziel

Fotos aus zeitstempel-basierten Ordnern optimal über Fotobuch-Seiten verteilen unter Berücksichtigung von:
- Chronologischer Reihenfolge (strikt)
- Aspect-Ratio-Respektierung (soft constraint)
- Gruppen-Kohäsion (benachbarte Gruppen dürfen gemischt werden)
- Konfigurierbarer Seitenzahl
- Ästhetischen Layout-Regeln

## Architektur-Übersicht

### Drei-Komponenten-System (Zwei Modi)

#### Modus A: CLI (Process-Based)
```
┌─────────────────────────────────────────────────────────┐
│                    Rust Core Binary                      │
│  - CLI Interface                                         │
│  - File I/O (EXIF, Scannen)                              │
│  - Python Environment Management                         │
│  - JSON Serialization                                    │
│  - Typst Export & PDF Compilation                        │
└────────────────┬────────────────────────────────────────┘
                 │
                 │ Subprocess: python -m photosolver solve input.json
                 ↓
┌─────────────────────────────────────────────────────────┐
│              Python Package (photosolver)                │
│  - Typer CLI (solve command)                             │
│  - FastAPI/Pydantic Models                               │
│  - OR-Tools CP-SAT Solver                                │
│  - JSON Input → Optimized Layout → JSON Output          │
└────────────────┬────────────────────────────────────────┘
                 │
                 │ Layout-JSON zurück via stdout
                 ↓
┌─────────────────────────────────────────────────────────┐
│                     Typst Export                         │
│  - Rust generiert .typ Source                            │
│  - Typst kompiliert zu PDF                               │
└─────────────────────────────────────────────────────────┘
```

#### Modus B: REST API (Server-Based, schneller)
```
┌─────────────────────────────────────────────────────────┐
│                    Rust Core Binary                      │
│  - HTTP Client (reqwest)                                 │
│  - POST /solve mit JSON                                  │
└────────────────┬────────────────────────────────────────┘
                 │
                 │ HTTP POST http://localhost:8000/solve
                 ↓
┌─────────────────────────────────────────────────────────┐
│         Python API Server (photosolver serve)            │
│  - FastAPI REST-Endpoints                                │
│  - OR-Tools Solver (persistent)                          │
│  - Kein Process-Overhead!                                │
└────────────────┬────────────────────────────────────────┘
                 │
                 │ JSON Response
                 ↓
┌─────────────────────────────────────────────────────────┐
│                  Typst Export (Rust)                     │
└─────────────────────────────────────────────────────────┘

Separates Terminal:
$ photosolver serve  # Server läuft persistent
```

## Projektstruktur

```
fotobuch/
├── Cargo.toml                  # Rust dependencies
├── pyproject.toml              # Python package config
├── uv.lock                     # Python dependency lock file
├── README.md
├── .gitignore                  # + .venv/, __pycache__/
│
├── src/
│   ├── main.rs                 # CLI entry point
│   ├── models.rs               # Rust Datenstrukturen (Photo, Page, Placement)
│   ├── scanner.rs              # Ordner scannen, EXIF lesen
│   ├── python_env.rs           # Python venv management (check + install from lock)
│   ├── solver.rs               # Solver-Orchestration
│   │                           #   - solve_heuristic() (Fallback)
│   │                           #   - solve_ortools_cli() (via process call)
│   │                           #   - solve_ortools_api() (via REST)
│   └── typst_export.rs         # Typst-Generierung & PDF-Kompilierung
│
├── python/
│   ├── pyproject.toml          # Python package metadata
│   ├── uv.lock                 # Locked dependencies (eingebacken in Rust)
│   ├── photosolver/
│   │   ├── __init__.py         # Package exports
│   │   ├── __main__.py         # CLI entry point (typer)
│   │   ├── cli.py              # Typer CLI commands (solve, serve)
│   │   ├── models.py           # FastAPI/Pydantic models
│   │   ├── api.py              # FastAPI REST API
│   │   ├── solver.py           # OR-Tools CP-SAT Solver
│   │   └── utils.py            # Helper functions
│   └── tests/
│       ├── test_solver.py
│       ├── test_api.py
│       └── test_data/
│           └── sample_input.json
│
├── test_photos/
│   ├── 2024-07-15_Urlaub/
│   └── 2024-08-20_Geburtstag/
│
└── output/
    ├── fotobuch.typ
    └── fotobuch.pdf
```

## Modul-Verantwortlichkeiten

### 1. Rust Core (`src/`)

#### `main.rs`
- CLI-Parsing (clap)
- Workflow-Orchestrierung:
  1. Python-Env sicherstellen
  2. Scanner aufrufen
  3. Solver aufrufen
  4. Typst-Export durchführen

#### `python_env.rs` ✨ **Neu**
```rust
pub struct PythonEnv {
    venv_path: PathBuf,
    python_exe: PathBuf,
}

impl PythonEnv {
    /// Prüft uv/Python-Verfügbarkeit und erstellt venv aus uv.lock
    /// Gibt Fehler aus, falls uv oder python3 nicht installiert sind.
    pub fn ensure() -> Result<Self>;
    
    /// Prüft ob uv installiert ist
    fn check_uv_installed() -> Result<()>;
    
    /// Prüft ob Python 3.10+ installiert ist
    fn check_python_installed() -> Result<()>;
    
    /// Erstellt venv mit uv sync basierend auf eingebackenem uv.lock
    fn create_venv_from_lock() -> Result<PathBuf>;
}
```

**Verhalten:**
- Prüft ob `uv` verfügbar ist → sonst klare Fehlermeldung + Installationsanleitung
- Prüft ob `python3` ≥3.10 verfügbar ist → sonst Fehlermeldung
- Prüft ob `.venv/` im Projektverzeichnis existiert
- Falls nicht:
  1. Schreibt eingebackenes `uv.lock` nach `.venv-setup/uv.lock`
  2. Führt `uv sync` aus (installiert exakte Versionen aus Lock)
  3. Verifiziert Installation (`python -c "import ortools"`)
- **User muss uv + Python selbst installiert haben!**

#### `scanner.rs`
- Ordner rekursiv scannen
- Zeitstempel aus Namen parsen (regex)
- EXIF-Daten lesen (Timestamp, Dimensionen)
- Gruppen chronologisch sortieren
- Photo-IDs generieren (z.B. hash oder relative Pfade)
- `area_weight` default auf 1.0 setzen (später via CLI/Config überschreibbar)

#### `solver.rs`
```rust
pub enum SolverMode {
    Heuristic,           // Eingebaute Heuristik
    OrtoolsCli,          // Python-Process: photosolver solve <file>
    OrtoolsRestApi,      // HTTP: POST /solve
}

pub fn solve(groups: &[PhotoGroup], config: &BookConfig, mode: SolverMode) -> Result<Vec<Page>> {
    match mode {
        SolverMode::Heuristic => solve_heuristic(groups, config),
        SolverMode::OrtoolsCli => solve_ortools_cli(groups, config),
        SolverMode::OrtoolsRestApi => solve_ortools_api(groups, config),
    }
}

fn solve_ortools_cli(groups: &[PhotoGroup], config: &BookConfig) -> Result<Vec<Page>> {
    let py_env = PythonEnv::ensure()?;
    
    // 1. Build photo ID → Photo mapping (for later reconstruction)
    let mut photo_map = HashMap::new();
    let api_photos: Vec<PhotoApiDto> = groups
        .iter()
        .flat_map(|g| &g.photos)
        .map(|p| {
            photo_map.insert(p.id.clone(), p.clone());
            PhotoApiDto::from(p)
        })
        .collect();
    
    // 2. Serialize input to JSON (WITHOUT paths)
    let input = json!({
        "photos": api_photos,
        "config": config,
    });
    let input_file = write_temp_json(&input)?;
    
    // 3. Call: photosolver solve <file.json>
    let output = Command::new(py_env.python_path())
        .args(["-m", "photosolver", "solve", &input_file])
        .output()?;
    
    if !output.status.success() {
        bail!("Solver failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // 4. Deserialize result
    let solver_output: SolverOutput = serde_json::from_slice(&output.stdout)?;
    
    // 5. Reconstruct Pages with full Photo objects (including paths)
    let pages = reconstruct_pages(solver_output.pages, &photo_map)?;
    
    Ok(pages)
}

fn reconstruct_pages(api_pages: Vec<ApiPage>, photo_map: &HashMap<String, Photo>) -> Result<Vec<Page>> {
    api_pages
        .into_iter()
        .map(|api_page| {
            let placements = api_page
                .placements
                .into_iter()
                .map(|p| {
                    let photo = photo_map
                        .get(&p.photo_id)
                        .ok_or_else(|| anyhow!("Photo ID not found: {}", p.photo_id))?
                        .clone();
                    
                    Ok(Placement {
                        photo,
                        x_mm: p.x_mm,
                        y_mm: p.y_mm,
                        width_mm: p.width_mm,
                        height_mm: p.height_mm,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            
            Ok(Page { placements })
        })
        .collect()
}

fn solve_ortools_api(groups: &[PhotoGroup], config: &BookConfig) -> Result<Vec<Page>> {
    // User muss Server extern gestartet haben!
    let api_url = env::var("PHOTOSOLVER_API_URL")
        .unwrap_or_else(|_| "http://localhost:8000".to_string());
    
    let input = serialize_solver_input(groups, config)?;
    
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(format!("{}/solve", api_url))
        .json(&input)
        .send()
        .context("Failed to connect to photosolver API")?;
    
    if !response.status().is_success() {
        bail!("API error: {}", response.text()?);
    }
    
    let output: SolverOutput = response.json()?;
    Ok(output.pages)
}

fn solve_heuristic(groups: &[PhotoGroup], config: &BookConfig) -> Result<Vec<Page>> {
    // Bisherige einfache Heuristik (Fallback)
    Ok(heuristic_layout(groups, config))
}
```

#### `models.rs`
```rust
#[derive(Serialize, Deserialize)]
pub struct Photo {
    pub id: String,               // Sent to API
    pub path: PathBuf,            // NOT sent to API, used internally
    pub timestamp: Option<NaiveDateTime>,
    pub dimensions: Option<(u32, u32)>,
    pub area_weight: f64,         // Relative area importance (default: 1.0)
}

#[derive(Serialize, Deserialize)]
pub struct PhotoGroup {
    pub label: String,
    pub timestamp: Option<NaiveDateTime>,
    pub photos: Vec<Photo>,
}

pub struct Page {
    pub placements: Vec<Placement>,
}

pub struct Placement {
    pub photo: Photo,
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}

pub struct BookConfig {
    pub page_width_mm: f64,
    pub page_height_mm: f64,
    pub margin_mm: f64,
    pub gap_mm: f64,
    pub max_photos_per_page: usize,
    pub target_pages: Option<usize>,  // NEU: Ziel-Seitenzahl
}
```

#### `typst_export.rs`
- Generiert `.typ`-Datei mit `#place()` Commands
- Kompiliert mit Typst-API zu PDF
- Unverändert zur bisherigen Version

---

### 2. Python Package (`python/photosolver/`)

#### Package-Struktur

```python
# python/pyproject.toml
[project]
name = "photosolver"
version = "0.1.0"
requires-python = ">=3.10"
dependencies = [
    "fastapi>=0.109",
    "pydantic>=2.0",
    "uvicorn>=0.27",
    "typer>=0.9",
    "ortools>=9.10",
]

[project.scripts]
photosolver = "photosolver.cli:app"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
```

#### `models.py` - FastAPI/Pydantic Models
```python
"""Data models for photobook solver input/output."""
from typing import Optional, List
from datetime import datetime
from pydantic import BaseModel, Field

class Photo(BaseModel):
    """A single photo with metadata."""
    id: str = Field(..., description="Unique identifier (filename or hash)")
    width: int = Field(..., gt=0, description="Image width in pixels")
    height: int = Field(..., gt=0, description="Image height in pixels")
    timestamp: Optional[datetime] = Field(None, description="Photo timestamp (EXIF or folder)")
    group: str = Field(..., description="Group label (folder name)")
    area_weight: float = Field(1.0, gt=0, description="Relative area weight (1.0 = default, 2.0 = double area)")
    
    @property
    def aspect_ratio(self) -> float:
        return self.width / self.height
    
    @property
    def is_landscape(self) -> bool:
        return self.width >= self.height

class BookConfig(BaseModel):
    """Configuration for photobook layout."""
    page_width_mm: float = Field(297.0, gt=0, description="Page width in mm")
    page_height_mm: float = Field(210.0, gt=0, description="Page height in mm")
    margin_mm: float = Field(10.0, ge=0, description="Page margin in mm")
    gap_mm: float = Field(3.0, ge=0, description="Gap between photos in mm")
    max_photos_per_page: int = Field(4, ge=1, le=10, description="Maximum photos per page")
    target_pages: Optional[int] = Field(None, ge=1, description="Target number of pages (soft constraint)")
    
    # Objective weights
    weight_aspect_ratio: float = Field(1.0, ge=0, description="Weight for aspect ratio preservation")
    weight_group_cohesion: float = Field(2.0, ge=0, description="Weight for keeping groups together")
    weight_page_count: float = Field(0.5, ge=0, description="Weight for target page count")

class Placement(BaseModel):
    """A photo placement on a page."""
    photo_id: str = Field(..., description="Reference to Photo.id")
    x_mm: float = Field(..., description="X offset from top-left in mm")
    y_mm: float = Field(..., description="Y offset from top-left in mm")
    width_mm: float = Field(..., gt=0, description="Placed width in mm")
    height_mm: float = Field(..., gt=0, description="Placed height in mm")

class Page(BaseModel):
    """A single page in the photobook."""
    page_number: int = Field(..., ge=1, description="Page number (1-indexed)")
    placements: List[Placement] = Field(default_factory=list, description="Photos on this page")

class SolverInput(BaseModel):
    """Complete input for the solver."""
    photos: List[Photo] = Field(..., description="All photos to place")
    config: BookConfig = Field(default_factory=BookConfig, description="Layout configuration")

class SolverOutput(BaseModel):
    """Solver output with optimized layout."""
    pages: List[Page] = Field(..., description="Optimized page layouts")
    statistics: dict = Field(default_factory=dict, description="Solver statistics")

class SolverStatus(BaseModel):
    """API status response."""
    status: str = Field(..., description="Solver status")
    version: str = Field(..., description="Package version")
```

#### `api.py` - FastAPI REST API
```python
"""REST API for photobook solver."""
from fastapi import FastAPI, HTTPException
from .models import SolverInput, SolverOutput, SolverStatus
from .solver import solve_ortools
import traceback

app = FastAPI(
    title="Photobook Solver API",
    description="OR-Tools based optimization for photobook layouts",
    version="0.1.0",
)

@app.get("/", response_model=SolverStatus)
async def root():
    """API status endpoint."""
    return SolverStatus(
        status="ready",
        version="0.1.0",
    )

@app.post("/solve", response_model=SolverOutput)
async def solve(input_data: SolverInput) -> SolverOutput:
    """Solve photobook layout optimization.
    
    Args:
        input_data: Photos and configuration
    
    Returns:
        Optimized page layouts
    
    Raises:
        HTTPException: If solving fails
    """
    try:
        result = solve_ortools(input_data)
        return result
    except Exception as e:
        raise HTTPException(
            status_code=500,
            detail=f"Solver failed: {str(e)}\n{traceback.format_exc()}"
        )

@app.get("/health")
async def health():
    """Health check endpoint."""
    return {"status": "healthy"}
```

#### `cli.py` - Typer CLI
```python
"""Command-line interface using Typer."""
import typer
import json
import sys
from pathlib import Path
from typing import Optional
from .models import SolverInput, SolverOutput
from .solver import solve_ortools
from .api import app as fastapi_app
import uvicorn

app = typer.Typer(
    name="photosolver",
    help="Photobook layout solver using OR-Tools",
)

@app.command()
def solve(
    input_file: Path = typer.Argument(..., help="Input JSON file"),
    output_file: Optional[Path] = typer.Option(None, "--output", "-o", help="Output JSON file (default: stdout)"),
    pretty: bool = typer.Option(False, "--pretty", help="Pretty-print JSON output"),
):
    """Solve a single photobook layout problem from JSON file.
    
    Example:
        photosolver solve input.json -o output.json --pretty
    """
    try:
        # Load input
        with open(input_file) as f:
            input_data = SolverInput.model_validate_json(f.read())
        
        # Solve
        typer.echo(f"Solving layout for {len(input_data.photos)} photos...", err=True)
        result = solve_ortools(input_data)
        typer.echo(f"✅ Generated {len(result.pages)} pages", err=True)
        
        # Output
        output_json = result.model_dump_json(indent=2 if pretty else None)
        
        if output_file:
            with open(output_file, 'w') as f:
                f.write(output_json)
            typer.echo(f"Written to {output_file}", err=True)
        else:
            print(output_json)
        
    except Exception as e:
        typer.echo(f"❌ Error: {e}", err=True)
        raise typer.Exit(1)

@app.command()
def serve(
    host: str = typer.Option("127.0.0.1", help="Bind host"),
    port: int = typer.Option(8000, help="Bind port"),
    reload: bool = typer.Option(False, help="Enable auto-reload"),
):
    """Start REST API server.
    
    Example:
        photosolver serve --host 0.0.0.0 --port 8080
    """
    typer.echo(f"🚀 Starting API server on {host}:{port}")
    typer.echo(f"📖 Docs: http://{host}:{port}/docs")
    
    uvicorn.run(
        "photosolver.api:app",
        host=host,
        port=port,
        reload=reload,
    )

@app.command()
def validate(
    input_file: Path = typer.Argument(..., help="Input JSON file to validate"),
):
    """Validate input JSON against schema.
    
    Example:
        photosolver validate input.json
    """
    try:
        with open(input_file) as f:
            input_data = SolverInput.model_validate_json(f.read())
        
        typer.echo(f"✅ Valid input: {len(input_data.photos)} photos", err=True)
        typer.echo(f"   Config: {input_data.config.page_width_mm}x{input_data.config.page_height_mm}mm", err=True)
        
    except Exception as e:
        typer.echo(f"❌ Validation failed: {e}", err=True)
        raise typer.Exit(1)

@app.command()
def schema(
    output_file: Optional[Path] = typer.Option(None, "--output", "-o", help="Output file (default: stdout)"),
):
    """Generate JSON schema for input/output formats.
    
    Example:
        photosolver schema -o schema.json
    """
    schema = {
        "input": SolverInput.model_json_schema(),
        "output": SolverOutput.model_json_schema(),
    }
    
    output_json = json.dumps(schema, indent=2)
    
    if output_file:
        with open(output_file, 'w') as f:
            f.write(output_json)
        typer.echo(f"Written to {output_file}", err=True)
    else:
        print(output_json)

if __name__ == "__main__":
    app()
```

#### `__main__.py` - Entry Point
```python
"""Entry point for python -m photosolver."""
from .cli import app

if __name__ == "__main__":
    app()
```

#### `solver.py` - OR-Tools Implementation
```python
"""OR-Tools CP-SAT based solver."""
from ortools.sat.python import cp_model
from .models import SolverInput, SolverOutput, Page, Placement
import time

def solve_ortools(input_data: SolverInput) -> SolverOutput:
    """Solve photobook layout using OR-Tools CP-SAT.
    
    Args:
        input_data: Photos and configuration
    
    Returns:
        Optimized page layouts
    """
    start_time = time.time()
    
    photos = input_data.photos
    config = input_data.config
    
    # Build model
    model = cp_model.CpModel()
    
    # Variables
    # TODO: Define decision variables
    #   - page_assignment[photo_id] -> page_number
    #   - x_position[photo_id] -> x coordinate (discretized)
    #   - y_position[photo_id] -> y coordinate (discretized)
    #   - width[photo_id] -> placed width (discretized)
    #   - height[photo_id] -> placed height (discretized)
    
    # Constraints
    # TODO: Implement constraints
    #   - Chronological order
    #   - No overlap (AddNoOverlap2D)
    #   - Area weight constraints:
    #       For photos on same page with weights [w1, w2, ...]:
    #       area_i ≈ total_usable_area * (w_i / sum(weights))
    #       Implement as soft constraint in objective
    
    # Objective
    # TODO: Minimize weighted sum of:
    #   - Aspect ratio deviation
    #   - Group splits
    #   - Page count deviation
    #   - Area weight deviation (new):
    #       For each page, minimize |actual_area_i - target_area_i|
    #       where target_area_i = usable_area * (weight_i / sum_weights)on
    #   - Group splits
    #   - Page count deviation
    
    # Solve
    solver = cp_model.CpSolver()
    solver.parameters.max_time_in_seconds = 30.0
    
    status = solver.Solve(model)
    
    solve_time = time.time() - start_time
    
    if status in (cp_model.OPTIMAL, cp_model.FEASIBLE):
        # Extract solution
        pages = extract_solution(solver, photos, config)
        
        return SolverOutput(
            pages=pages,
            statistics={
                "status": solver.StatusName(status),
                "solve_time_seconds": solve_time,
                "num_photos": len(photos),
                "num_pages": len(pages),
            }
        )
    else:
        raise RuntimeError(f"No solution found: {solver.StatusName(status)}")

def extract_solution(solver: cp_model.CpSolver, photos, config) -> list[Page]:
    """Extract page layouts from solved model."""
    # TODO: Implement solution extraction
    pages = []
    return pages
```

---

### 3. Python Environment Setup

#### Python CLI Commands (via Typer)

```bash
# Direktes Lösen einer JSON-Datei
photosolver solve input.json -o output.json --pretty

# REST-API Server starten
photosolver serve --host 0.0.0.0 --port 8080 --reload

# Input validieren
photosolver validate input.json

# JSON-Schema exportieren
photosolver schema -o schema.json
```

**Beispiel-Workflow:**
```bash
# 1. Schema generieren für Rust-Integration
cd python/
uv run photosolver schema -o ../schema.json

# 2. Test-Input erstellen (manuell oder via Rust-Scanner)
cat > test_input.json << EOF
{
  "photos": [...],
  "config": {...}
}
EOF

# 3. Validieren
uv run photosolver validate test_input.json
# ✅ Valid input: 10 photos

# 4. Lösen
uv run photosolver solve test_input.json --pretty
# [JSON Output mit optimiertem Layout]

# 5. API-Server für Development
uv run photosolver serve --reload
# 🚀 Starting API server on 127.0.0.1:8000
# 📖 Interactive docs: http://127.0.0.1:8000/docs
```

#### Voraussetzungen (User-Installation erforderlich)

**User muss folgendes installiert haben:**
1. **Python 3.10+**
   ```bash
   python3 --version  # sollte ≥3.10 sein
   ```

2. **uv (Python Package Manager)**
   ```bash
   curl -LsSf https://astral.sh/uv/install.sh | sh
   # oder: brew install uv
   ```

#### Setup-Flow (automatisiert durch Rust)

```
User startet: ./photobook-solver --input photos/
                      ↓
         PythonEnv::ensure() wird aufgerufen
                      ↓
         ┌─────────────────────────────────┐
         │ 1. Check: uv installiert?       │
         │    ❌ → Fehlermeldung + Link    │
         ├─────────────────────────────────┤
         │ 2. Check: python3 ≥3.10?        │
         │    ❌ → Fehlermeldung + Link    │
         ├─────────────────────────────────┤
         │ 3. Check: .venv/ existiert?     │
         │    ❌ → Erstellen                │
         ├─────────────────────────────────┤
         │ 4. uv.lock ins Binary einbacken │
         │    → bei Build-Zeit included    │
         ├─────────────────────────────────┤
         │ 5. uv sync --frozen             │
         │    (installiert exakte Versionen│
         │     aus eingebackenem Lock)     │
         ├─────────────────────────────────┤
         │ 6. Verify: import ortools       │
         └─────────────────────────────────┘
                      ↓
         ✅ Python Environment bereit
```

#### Einbacken des uv.lock (Build-Time)

```rust
// build.rs
use std::fs;

fn main() {
    // uv.lock ins Binary einbacken
    let lock_content = fs::read_to_string("python/uv.lock")
        .expect("uv.lock not found - run 'uv lock' first");
    
    println!("cargo:rustc-env=UV_LOCK_CONTENT={}", lock_content);
    println!("cargo:rerun-if-changed=python/uv.lock");
}
```

```rust
// src/python_env.rs
const UV_LOCK: &str = env!("UV_LOCK_CONTENT");

impl PythonEnv {
    fn create_venv_from_lock() -> Result<PathBuf> {
        let venv_path = PathBuf::from(".venv");
        
        // uv.lock temporär schreiben
        let lock_file = venv_path.join("uv.lock");
        fs::create_dir_all(&venv_path)?;
        fs::write(&lock_file, UV_LOCK)?;
        
        // uv sync --frozen (reproduzierbare Installation)
        let status = Command::new("uv")
            .args(["sync", "--frozen"])
            .current_dir(&venv_path.parent().unwrap())
            .status()?;
        
        if !status.success() {
            bail!("uv sync failed");
        }
        
        Ok(venv_path)
    }
}
```

#### Fehlermeldungen

```rust
// Wenn uv nicht gefunden
"""
❌ Error: 'uv' not found

Please install uv:
  • Linux/Mac:   curl -LsSf https://astral.sh/uv/install.sh | sh
  • Windows:     powershell -c "irm https://astral.sh/uv/install.ps1 | iex"
  • Homebrew:    brew install uv

See: https://docs.astral.sh/uv/getting-started/installation/
"""

// Wenn Python nicht gefunden oder zu alt
"""
❌ Error: Python 3.10+ not found

Please install Python 3.10 or newer:
  • Ubuntu/Debian:  sudo apt install python3
  • macOS:          brew install python3
  • Windows:        Download from python.org

Current: {detected_version or "not found"}
Required: ≥3.10
"""
```

#### Dependencies

```toml
# Cargo.toml
[package]
name = "photobook-solver"
version = "0.1.0"

[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
chrono = "0.4"
kamadak-exif = "0.6"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "5"           # Platform-aware paths
reqwest = { version = "0.11", features = ["blocking", "json"] }  # For REST API calls
typst = "0.14"
typst-pdf = "0.14"

[build-dependencies]
# Für uv.lock einbacken
```

```toml
# python/pyproject.toml
[project]
name = "photosolver"
version = "0.1.0"
requires-python = ">=3.10"
dependencies = [
    "fastapi>=0.109",
    "pydantic>=2.0",
    "uvicorn>=0.27",
    "typer>=0.9",
    "ortools>=9.10",
]

[project.scripts]
photosolver = "photosolver.cli:app"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.uv]
dev-dependencies = [
    "pytest>=7.0",
    "httpx>=0.26",  # For testing FastAPI
]
```

---

## JSON-Schnittstellenformat

Das JSON-Format wird **automatisch aus den FastAPI/Pydantic-Modellen** generiert.

### Schema exportieren

```bash
photosolver schema -o schema.json
```

### Input (Rust → Python)

```json
{
  "photos": [
    {width": 4032,
      "height": 3024,
      "timestamp": "2024-07-15T14:30:00",
      "group": "2024-07-15_Urlaub",
      "area_weight": 1.0
    },
    {
      "id": "photo_002",
      "width": 3024,
      "height": 4032,
      "timestamp": "2024-07-15T15:45:00",
      "group": "2024-07-15_Urlaub",
      "area_weight": 2.0
    },
    {
      "id": "photo_003",
      "width": 4032,
      "height": 3024,
      "timestamp": "2024-07-15T16:20:00",
      "group": "2024-07-15_Urlaub",
      "area_weight": 1.0
      "timestamp": "2024-07-15T15:45:00",
      "group": "2024-07-15_Urlaub"
    }
  ],
  "config": {
    "page_width_mm": 297.0,
    "page_height_mm": 210.0,
    "margin_mm": 10.0,
    "gap_mm": 3.0,
    "max_photos_per_page": 4,
    "target_pages": 20,
    "weight_aspect_ratio": 1.0,
    "weight_group_cohesion": 2.0,
    "weight_page_count": 0.5
  }
}
```

### Output (Python → Rust)

```json
{
  "pages": [
    {
      "page_number": 1,
      "placements": [
        {
          "photo_id": "photo_001",
          "x_mm": 10.0,
          "y_mm": 10.0,
          "width_mm": 135.5,
          "height_mm": 190.0
        },
        {
          "photo_id": "photo_002",
          "x_mm": 148.5,
          "y_mm": 10.0,
          "width_mm": 138.5,
          "height_mm": 190.0
        }
      ]
    }
  ],
  "statistics": {
    "status": "OPTIMAL",
    "solve_time_seconds": 2.45,
    "num_photos": 47,
    "num_pages": 12
  }
}
```

### Validierung

```bash
# Input validieren
photosolver validate input.json

# Ausgabe:
# ✅ Valid input: 47 photos
#    Config: 297.0x210.0mm
```

---

## Optimierungsziele (Constraints & Objective)

### Hard Constraints (MÜSSEN erfüllt sein)

1. **Chronologische Reihenfolge**
   - Foto mit früherer Timestamp kommt auf früherer/gleicher Seite
   - Keine Umordnung innerhalb Gruppen

2. **Keine Überlappung**
   - Photos dürfen sich nicht überlappen (2D NoOverlap)
   - Respektiere Ränder und Gaps

3. **Gruppen-Angrenzung**
   - Verschiedene Gruppen dürfen nur gemischt werden, wenn chronologisch angrenzend
   - Frühere Gruppe links von späterer (bei Mischung auf gleicher Seite)

### Soft Constraints (Objective minimieren)

1. **Aspect-Ratio-Abweichung**
   - Minimiere Differenz zwischen Original- und Platzierungs-Aspect-Ratio
   - Gewichtung: Mittel

2. **Area-Weight Zuweisung** ✨ Neu
   - Fotos mit höherem `area_weight` sollen mehr Fläche bekommen
   - Beispiel: 3 Fotos auf Seite mit Gewichten [1, 1, 2]
     - Foto 1: ~25% der nutzbaren Fläche (1/4)
     - Foto 2: ~25% der nutzbaren Fläche (1/4)
     - Foto 3: ~50% der nutzbaren Fläche (2/4)
   - Berechnung: `target_area_i = usable_area * (weight_i / sum(weights_on_page))`
   - **Wichtig:** Gilt nur für Fotos **auf derselben Seite**
   - Implementation: Soft-Constraint (penalty für Abweichung vom Target)
   - Gewichtung: Mittel-Hoch

3. **Gruppen-Kohäsion**
   - Bevorzuge Gruppen komplett auf einer Seite
   - Penalty für Gruppen-Splits
   - Gewichtung: Hoch

4. **Seitenzahl-Ziel**
   - Wenn `target_pages` gesetzt: bevorzuge diese Anzahl
   - Penalty für Abweichung
   - Gewichtung: Niedrig

5. **Ästhetische Balance**
   - Bevorzuge ausgeglichene Layouts
   - Gewichtung: Niedrig
   - Bevorzuge ausgeglichene Layouts
   - Gewichtung: Niedrig

---

## Workflow

### Entwicklungs-Workflow

```bash
# 1. Python-Env erstellen (einmalig)
cd python/
uv sync  # Erstellt .venv aus uv.lock

# 2. Python-Solver isoliert testen
cd python/
uv run photosolver solve ../test_photos/input.json --pretty

# 3. Python REST-API starten (für schnelles Testen)
uv run photosolver serve
# → Server läuft auf http://localhost:8000
# → Docs: http://localhost:8000/docs

# 4. Rust-Code entwickeln
cd ..
cargo run -- --input test_photos/ --solver api
# → Verwendet laufenden API-Server

# 5. Rust-Code mit CLI-Modus testen
cargo run -- --input test_photos/ --solver cli
# → Startet Python-Process für jeden Aufruf
```

### User-Workflow (Zwei Modi)

#### Modus A: CLI (Standard, einfach)
```bash
# Prerequisites: uv und python3 installiert
./photobook-solver --input photos/

# Beim ersten Start:
# ⏳ Creating Python environment (30 seconds)...
# ✅ Environment ready!
# 
# Scanning photos/ ...
# Found 47 photos in 3 groups
# Solving layout...
# Generated 12 pages
# ✅ fotobuch.pdf created

# Nachfolgende Starts: instant
```

#### Modus B: REST API (schneller für mehrere Runs)

Terminal 1 - Server starten:
```bash
cd python/
uv run photosolver serve
# 🚀 Starting API server on 127.0.0.1:8000
# 📖 Docs: http://127.0.0.1:8000/docs
```

Terminal 2 - Photobook-Solver verwenden:
```bash
export PHOTOSOLVER_API_URL=http://localhost:8000
./photobook-solver --input photos/ --solver api

# Viel schneller: kein Python-Process-Startup!
```

### CLI-Flags

```bash
photobook-solver --help

Options:
  --input <DIR>           Input directory with timestamped photo folders
  --output <FILE>         Output PDF file [default: photobook.pdf]
  --solver <MODE>         Solver mode: heuristic|cli|api [default: cli]
  --page-width <MM>       Page width in mm [default: 297.0]
  --page-height <MM>      Page height in mm [default: 210.0]
  --margin <MM>           Margin in mm [default: 10.0]
  --gap <MM>              Gap between photos in mm [default: 3.0]
  --max-photos <N>        Max photos per page [default: 4]
  --target-pages <N>      Target number of pages (soft constraint)
  --default-area-weight   Default area weight for all photos [default: 1.0]
  
Environment:
  PHOTOSOLVER_API_URL     API endpoint for --solver api mode

Future:
  --area-weight-file      JSON file mapping photo IDs to area weights
                          Example: {"photo_001": 2.0, "photo_005": 1.5}
```

### Build & Distribution

#### Prerequisites für Build

```bash
# 1. Python-Dependencies locken
cd python/
uv lock  # Erzeugt uv.lock

# 2. Rust-Build (uv.lock wird eingebacken)
cd ..
cargo build --release

# Binary: target/release/photobook-solver (~5-8 MB)
```

#### Distribution

**Was der User bekommt:**
- Ein Binary: `photobook-solver` / `photobook-solver.exe`
- Binary enthält eingebackenes `uv.lock`
- Größe: ~5-8 MB

**Was der User braucht:**
1. Python 3.10+ installiert
2. `uv` installiert (einmalig)
3. Internet für erste Installation (Dependencies aus `uv.lock`)

**Erste Verwendung:**
```bash
# User lädt nur das Binary herunter
./photobook-solver --input photos/

# Beim ersten Start:
# ⏳ Setting up Python environment...
#    (Downloads ~50 MB OR-Tools, einmalig)
# ✅ Ready!
# [normaler Output]

# Ab jetzt: Instant-Start
```

#### GitHub Release

```bash
# Build für alle Plattformen (via CI)
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-pc-windows-msvc
cargo build --release --target aarch64-apple-darwin

# Releases:
photobook-solver-v1.0-linux-x86_64
photobook-solver-v1.0-windows-x86_64.exe
photobook-solver-v1.0-macos-aarch64
```

**README für User:**
```markdown
## Installation

1. Download the binary for your platform
2. Install prerequisites:
   - Python 3.10+:  https://python.org
   - uv:            curl -LsSf https://astral.sh/uv/install.sh | sh
3. Run: ./photobook-solver --input photos/
   (First run sets up environment automatically)
```

---

## Nächste Schritte

### Phase 1: Python Package Setup
- [ ] `python/pyproject.toml` erstellen
- [ ] FastAPI-Modelle in `models.py` implementieren
  - [ ] `Photo` mit `id`, `area_weight` (kein `path`)
  - [ ] `BookConfig` mit allen Weights
  - [ ] `Placement` mit `photo_id`-Referenz
- [ ] Typer CLI in `cli.py` implementieren
- [ ] FastAPI REST-API in `api.py` implementieren
- [ ] Stub-Solver mit Mock-Daten testen
- [ ] `uv lock` ausführen → `uv.lock` erzeugen

### Phase 2: Rust Integration
- [ ] `build.rs` für uv.lock-Einbettung
- [ ] `python_env.rs` implementieren:
  - [ ]models.rs` erweitern:
  - [ ] Photo mit `id`, `path`, `area_weight`
  - [ ] `PhotoApiDto` (ohne path)
  - [ ] Konvertierung Photo → PhotoApiDto
- [ ] `scanner.rs` anpassen:
  - [ ] Photo-IDs generieren
  - [ ] `area_weight` Standard auf 1.0
  - [ ] Optional: area_weight aus Dateinamen/Metadaten lesen
- [ ] `solver.rs` erweitern:
  - [ ] CLI-Modus mit photo_map (ID → Photo)
  - [ ] API-Modus mit photo_map
  - [ ] `reconstruct_pages()` Helper-Function
- [ ] `solver.rs` erweitern:
  - [ ] CLI-Modus (Process-Call)
  - [ ] API-Modus (REST-Call)
  - [ ] Mode-Selection in CLI
- [ ] JSON-Serialisierung anpassen (match FastAPI models)
- [ ] Reqwest für REST-API-Calls einbinden

### Phase 3: OR-Tools Solver (Python)
- [ ] Basis-Modell mit CP-SAT
- [ ] Variablen definieren:
  - [ ] **Area-Weight-Deviation** (neu):
    - [ ] Berechne Ziel-Flächen pro Foto basierend auf weights
    - [ ] Minimize |actual_area - target_area|
- [ ] Solution-Extraction mit area-Statistike
  - [ ] 2D-Platzierung (diskretisiert)
  - [ ] Größen-Variablen
- [ ] Hard Constraints:
  - [ ] Chronologische Reihenfolge
  - [ ] No Overlap (AddNoOverlap2D oder custom)
  - [ ] Gruppen-Angrenzung
  - [ ] Seitenränder
- [ ] Objective Function:
  - [ ] Aspect-Ratio-Deviation
  - [ ] Group-Cohesion-Penalty
  - [ ] Page-Count-Deviation
- [ ] Solution-Extraction

### Phase 4: Testing & Integration
- [ ] Python Unit-Tests (pytest)
  - [ ] Modell-Validierung
  - [ ] API-Endpoints
  - [ ] Solver mit kleinen Test-Cases
- [ ] Rust Integration-Tests
  - [ ] CLI-Modus
  - [ ] API-Modus
  - [ ] Fallback zu Heuristik
- [ ] End-to-End Tests mit echten Fotos
- [ ] Performance-Messungen

### Phase 5: Distribution & Dokumentation
- [ ] README mit Setup-Anleitung
- [ ] API-Dokumentation (automatisch via FastAPI)
- [ ] Solver-Parameter-Guide
- [ ] CI/CD Setup (GitHub Actions)
  - [ ] Multi-Platform Builds
  - [ ] Automated Tests
  - [ ] Release Management
- [ ] Binary-Releases für Linux/macOS/Windows

---

## Offene Fragen & Design-Entscheidungen

### Zu klären:

1. **OR-Tools Constraint-Modellierung:**
   - Wie modellieren wir 2D-Intervall-Platzierung in CP-SAT?
   - Diskretisierung: Welche Granularität (0.1mm, 1mm, ...)?
   - Nutzen wir `AddNoOverlap2D` oder eigene Constraints?
   - Wie repräsentieren wir variable Foto-Gr

6. **Area-Weight Implementation:**
   - Exakte Umsetzung: Hard constraint oder soft im Objective?
   - Toleranz für Abweichung vom Ziel-Area?
   - Was wenn area_weights und andere Constraints konfligieren?
   - Default-Gewichtung für area_weight in Objective-Function?ößen?

2. **Gruppen-Angrenzung:**
   - Genauer Constraint: "Gruppe A links von Gruppe B" auf einer Seite?
   - Erlaubt: Gruppe über mehrere Seiten verteilt, solange chronologisch?
   - Was wenn Gruppe zu groß für eine Seite?

3. **Performance & Skalierung:**
   - Timeout für Solver? (Standard: 30 Sekunden?)
   - Bei Timeout: Beste gefundene Lösung oder Fallback?
   - Ab wie vielen Fotos wird es kritisch? (50? 100? 500?)
   - Incrementelles Solving für große Mengen?

4. **API-Server Management:**
   - Soll Rust optional den API-Server selbst starten (Background-Process)?
   - Oder immer explizit vom User gestartet?
   - Health-Check vor API-Calls?
   - Timeout für API-Requests?

✅ **Kein path in API** - Nur photo_id wird übertragen, Rust hält path-Mapping  
✅ **area_weight pro Foto** - Relative Flächenzuweisung (default: 1.0)  
5. **Aspect-Ratio Handling:**
   - "Darf verletzt werden" → Wie stark?
   - Min/Max-Bounds für Deviation (z.B. ±20%)?
   - Cropping erlaubt oder nur Letterboxing?

6. **Config-Presets:**
   - Vordefinierte Formate (Saal Digital, CEWE, etc.)?
   - Config-File-Support (.toml/.json)?
   - Template-System für Layouts?

7. **Testing:**
   - Wie generieren wir reproduzierbare Test-Cases?
   - Mock-Photos oder echte Beispiel-Bilder im Repo?
   - Performance-Benchmarks definieren?

8. **Error-Handling:**
   - Was tun bei Python-Installation-Fehler?
   - Offline-Mode möglich? (mit vorinstalliertem venv)
   - Diagnostics/Logs für Debugging?
   - User-freundliche Fehlermeldungen vs. Debug-Info?

### Entscheidungen getroffen:

✅ **Python nicht automatisch installiert** - User-Verantwortung, klare Fehler  
✅ **uv.lock eingebacken** - Reproduzierbare Builds  
✅ **Zwei Solver-Modi** - CLI (einfach) + API (schnell)  
✅ **FastAPI für Modelle** - Automatisches Schema, API-Docs gratis  
✅ **Typer für CLI** - Type-safe, benutzerfreundlich  

---

## Referenzen

- **OR-Tools:** https://developers.google.com/optimization
- **CP-SAT Solver:** https://developers.google.com/optimization/cp/cp_solver
- **uv (Python Package Manager):** https://github.com/astral-sh/uv
- **Typst:** https://typst.app
