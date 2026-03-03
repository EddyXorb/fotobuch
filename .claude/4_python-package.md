# 4. Python-Package Details

## Module

- **`models.py`**: Pydantic-Models (Photo, BookConfig, Page, Placement, SolverInput/Output)
- **`cli.py`**: Typer CLI (`solve`, `serve`, `validate`, `schema`)
- **`api.py`**: FastAPI REST-Endpoints (`/solve`, `/health`)
- **`solver.py`**: OR-Tools CP-SAT Implementation
- **`__main__.py`**: Entry point für `python -m photosolver`

---

## models.py - Pydantic Models

```python
from pydantic import BaseModel, Field
from datetime import datetime

class Photo(BaseModel):
    """Photo ohne path - nur ID und Metadaten"""
    id: str
    width: int
    height: int
    timestamp: datetime | None = None
    group: str
    area_weight: float = Field(default=1.0, gt=0)

class BookConfig(BaseModel):
    page_width_mm: float
    page_height_mm: float
    margin_mm: float
    gap_mm: float
    max_photos_per_page: int
    target_pages: int | None = None
    max_aspect_deviation: float = Field(default=0.2)  # 20%
    timeout_seconds: int = Field(default=30, gt=0)
    
    # Objective Weights (alle via CLI/API anpassbar)
    weight_aspect_ratio: float = Field(default=1.0)
    weight_area: float = Field(default=10.0)  # Starke Bestrafung
    weight_group_cohesion: float = Field(default=2.0)
    weight_page_count: float = Field(default=0.5)

class Placement(BaseModel):
    photo_id: str
    x_mm: float
    y_mm: float
    width_mm: float
    height_mm: float

class Page(BaseModel):
    page_number: int
    placements: list[Placement]

class SolverInput(BaseModel):
    photos: list[Photo]
    config: BookConfig

class SolverOutput(BaseModel):
    pages: list[Page]
    statistics: dict
```

---

## solver.py - OR-Tools Implementation

**Key Points:**

```python
from ortools.sat.python import cp_model
import logging
import os
from pathlib import Path

# Logging Setup
def setup_logging():
    session_id = os.getenv('PHOTOSOLVER_SESSION_ID', 'unknown')
    debug = os.getenv('PHOTOSOLVER_DEBUG', '0') == '1'
    
    if debug:
        timestamp = __import__('datetime').datetime.now().strftime('%Y%m%d_%H%M%S')
        log_file = Path('output') / f"{timestamp}_{session_id}_python.log"
        log_file.parent.mkdir(exist_ok=True)
        
        logging.basicConfig(
            level=logging.DEBUG,
            format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
            handlers=[
                logging.FileHandler(log_file),
                logging.StreamHandler()  # Auch stderr
            ]
        )
        logging.info(f"Python solver session: {session_id}")
    else:
        logging.basicConfig(level=logging.INFO)

setup_logging()
logger = logging.getLogger(__name__)

def solve(input: SolverInput) -> SolverOutput:
    logger.info(f"Starting solver with {len(input.photos)} photos")
    logger.debug(f"Config: {input.config}")
    model = cp_model.CpModel()
    
    # Diskretisierung: 1mm
    # → Alle Koordinaten als Integer (mm)
    
    # Variables
    # - page_assignment[photo] → page_number
    # - x[photo], y[photo], w[photo], h[photo]
    
    # Hard Constraints
    # 1. Chronologisch: earlier timestamp → earlier/same page
    # 2. AddNoOverlap2D() für alle Fotos auf gleicher Seite
    # 3. Gruppen-Angrenzung: max(x in A) < min(x in B)
    
    # Soft Constraints (Objective) - ALLE Gewichtungen anpassbar
    # - Aspect-Ratio Deviation (max: max_aspect_deviation, no letterboxing)
    # - Area-Weight: |actual_area - target_area| mit ±25% Toleranz
    #   → Starke Bestrafung (weight_area = 10.0 default)
    # - Group Cohesion: penalty für splits
    # - Page Count: penalty für Abweichung von target_pages
    
    solver = cp_model.CpSolver()
    solver.parameters.max_time_in_seconds = input.config.timeout_seconds
    
    logger.info("Starting CP-SAT solver...")
    logger.debug(f"Timeout: {input.config.timeout_seconds}s")
    
    # WICHTIG: Auch bei Timeout beste Lösung zurückgeben
    status = solver.Solve(model)
    
    logger.info(f"Solver status: {solver.StatusName(status)}")
    logger.debug(f"Solve time: {solver.WallTime()}s")
    logger.debug(f"Objective value: {solver.ObjectiveValue()}")
    
    if status in [cp_model.OPTIMAL, cp_model.FEASIBLE]:
        return extract_solution(solver, ...)
    else:
        # Bei Konflikten klare Fehlermeldung
        error_msg = f"No solution found: {solver.StatusName(status)}\n"
        error_msg += "Try relaxing constraints:\n"
        error_msg += "  - Increase --max-aspect-deviation\n"
        error_msg += "  - Reduce area_weight differences\n"
        error_msg += "  - Increase --target-pages\n"
        raise ValueError(error_msg)
```

---

## Key Points

✅ Photo-Model **ohne `path`** - nur ID und Metadaten  
✅ `area_weight` Parameter pro Foto (default: 1.0)  
✅ `max_aspect_deviation` in Config (default: 0.2)  
✅ `timeout_seconds` in Config (default: 30)  
✅ Diskretisierung: **1mm** (Integer-Variablen)  
✅ `AddNoOverlap2D` für No-Overlap Constraint  
✅ Beste Lösung auch bei Timeout zurückgeben  
✅ FastAPI generiert automatisch `/docs` Endpoint  
✅ Type-safe mit Pydantic  
✅ Modernes Python (≥3.10)  
✅ **Vollständige Type Annotations** - Alle Funktionen haben Typ-Hinweise  
✅ **Linting** - ruff + mypy für Code-Qualität  

---

## Dependencies

- **fastapi** - REST API Framework
- **pydantic** - Data validation
- **uvicorn** - ASGI server
- **typer** - CLI framework
- **ortools** - Constraint solver (≥9.10)
- **python-multipart** - Für FastAPI file uploads (optional)

## Testing

**⚠️ WICHTIG: Coverage > 90% halten!**

```bash
cd python/

# Alle Tests mit Coverage
uv run pytest --cov=photosolver --cov-report=term --cov-report=html

# HTML-Report öffnen
open htmlcov/index.html

# Nur bestimmte Tests
uv run pytest tests/test_solver.py -v
```

**Test-Struktur:**
```
python/
├── photosolver/
│   ├── __init__.py
│   ├── models.py
│   ├── solver.py
│   └── ...
└── tests/
    ├── __init__.py
    ├── test_models.py
    ├── test_solver.py
    ├── test_api.py
    └── conftest.py  # Fixtures
```

➡️ [5. Environment-Setup](5_environment-setup.md)
