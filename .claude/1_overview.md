# 1. Projektziel & Architektur

## Projektziel

Fotos aus zeitstempel-basierten Ordnern optimal über Fotobuch-Seiten verteilen unter Berücksichtigung von:
- **Chronologischer Reihenfolge** (strikt)
- **Aspect-Ratio-Respektierung** (soft constraint)
- **Gruppen-Kohäsion** (benachbarte Gruppen dürfen gemischt werden)
- **Konfigurierbarer Seitenzahl**
- **Area-Weight pro Foto** (relative Flächenzuweisung)
- **Ästhetischen Layout-Regeln**

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

**Vorteile:**
- Einfach zu testen
- Python-Solver isoliert aufrufbar
- Klare Trennung der Verantwortlichkeiten

**Nachteile:**
- Process-Startup-Overhead (~100-200ms)

---

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

**Vorteile:**
- Kein Process-Overhead
- Schneller für mehrere Runs
- OR-Tools bleibt im Speicher

**Nachteile:**
- Server muss separat gestartet werden
- Zusätzliche Komplexität

---

## Komponenten-Verantwortlichkeiten

### Rust Core
- **CLI-Interface** mit clap
- **File I/O**: Fotos scannen, EXIF-Daten lesen
- **Python Environment Management**: venv prüfen/erstellen
- **JSON-Serialisierung**: Input für Python, Output parsen
- **Typst-Export**: .typ generieren & zu PDF kompilieren
- **Path-Mapping**: Photo-IDs ↔ Dateipfade (nicht in API übertragen)

### Python Solver (photosolver)
- **FastAPI/Pydantic Models**: Type-safe JSON-Schema
- **Typer CLI**: `solve`, `serve`, `validate`, `schema` Commands
- **OR-Tools CP-SAT Solver**: Constraint Programming
- **REST API**: FastAPI Endpoints mit Auto-Docs

### Typst
- **Layout-Rendering**: .typ → PDF
- **Präzise Platzierung**: mm-genaue Positionierung
- **Font-Handling**: System-Fonts

---

## Datenfluss

### CLI-Modus
```
1. Rust scannt Fotos → creates Photo objects (id, path, dimensions, area_weight)
2. Rust generiert JSON (ohne paths!) → temp file
3. Rust ruft auf: python -m photosolver solve temp.json
4. Python löst Optimization → gibt JSON zurück (photo_id, x, y, width, height)
5. Rust rekonstruiert Pages (photo_id → full Photo mit path)
6. Rust generiert .typ & kompiliert zu PDF
```

### API-Modus  
```
1-2. Wie oben
3. Rust macht HTTP POST /solve mit JSON
4. Python API antwortet mit JSON
5-6. Wie oben
```

---

## Nächstes Dokument

➡️ [2. Projektstruktur](2_projektstruktur.md) - Dateiverzeichnisse und Organisation
