# 9. Roadmap & Offene Fragen

## Nächste Schritte

### Phase 1: Python Package ✅ Setup
- [ ] `pyproject.toml` erstellen
- [ ] FastAPI-Modelle (`Photo` ohne `path`, + `area_weight`)
- [ ] Typer CLI (`solve`, `serve`, `validate`, `schema`)
- [ ] FastAPI REST-API
- [ ] Stub-Solver mit Mock-Daten
- [ ] `uv lock` ausführen

### Phase 2: Rust Integration
- [ ] `build.rs` für uv.lock-Embedding
- [ ] `python_env.rs` (check + install)
- [ ] `models.rs` (`Photo`, `PhotoApiDto`, `area_weight`)
- [ ] `scanner.rs` (Photo-IDs, area_weight=1.0)
- [ ] `solver.rs` (CLI/API-Modi, photo_map, reconstruct_pages())
- [ ] JSON-Serialisierung
- [ ] Reqwest für API-Calls

### Phase 3: OR-Tools Solver
- [ ] CP-SAT Basis-Modell
- [ ] Variablen (page_assignment, x/y, width/height)
- [ ] Hard Constraints (chronologisch, no-overlap, groups)
- [ ] Objective (aspect-ratio, group-cohesion, **area-weight** ✨)
- [ ] Solution-Extraction

### Phase 4: Testing
- [ ] Artificial Input Generator (siehe unten)
  - [ ] `test_photos/artificial_input_generator.py` ✅ Erstellt
  - [ ] Typer CLI: Gruppen, Fotos min/max, Ausgabeort ✅
  - [ ] Zufällige JPGs mit Farben & Größen ✅
- [ ] Python Code Quality
  - [ ] **Type Annotations** - Alle Funktionen typisiert
  - [ ] **mypy strict mode** - Type checking
  - [ ] **ruff** - Linting (E, F, I, N, W, UP, ANN)
  - [ ] **ruff format** - Auto-formatting
- [ ] Python Unit-Tests (pytest)
  - [ ] `test_models.py` - Pydantic validation
  - [ ] `test_solver.py` - OR-Tools constraints
  - [ ] `test_api.py` - FastAPI endpoints
  - [ ] **Coverage > 90%** sicherstellen!
- [ ] Rust Unit-Tests
  - [ ] `models.rs` - Serialization
  - [ ] `scanner.rs` - EXIF parsing
  - [ ] `solver.rs` - JSON communication
  - [ ] **Coverage > 90%** sicherstellen!
- [ ] Integration-Tests
  - [ ] End-to-End mit Mock-Fotos
  - [ ] CLI + API Modi
  - [ ] Debug-Output validieren
- [ ] Reproduzierbare Test-Cases
- [ ] Performance-Messungen (>200 Fotos)
- [ ] Coverage-Tools: `cargo-tarpaulin` (Rust), `pytest-cov` (Python)

### Phase 5: Distribution
- [ ] README
- [ ] API-Docs (auto via FastAPI)
- [ ] CI/CD (GitHub Actions)
  - [ ] Tests auf allen Branches
  - [ ] Coverage-Check (> 90%)
  - [ ] **Python: mypy + ruff check**
  - [ ] **Rust: clippy + rustfmt check**
  - [ ] Conventional Commits validieren
  - [ ] Build für Linux/macOS/Windows
- [ ] Multi-Platform Releases
- [ ] Release-Notes (aus Conventional Commits generieren)

---

## Offene Fragen

### 1. ~~OR-Tools Constraint-Modellierung~~ ✅ GEKLÄRT
- ✅ Diskretisierung: **1mm reicht**
- ✅ `AddNoOverlap2D` verwenden
- ✅ Variable Foto-Größen: Input-Dimensionen werden NICHT respektiert
  - Große Fotos kommen rein, Solver bestimmt individuelle Größe
  - Basierend auf: Seitengröße + Anzahl Bilder + Gewichte

### 2. ~~Gruppen-Angrenzung~~ ✅ GEKLÄRT
- ✅ Constraint: `max(x_start in A) < min(x_start in B)`
  - Rechtester oberer linker Startpunkt von A weiter links als linkester von B
- ✅ Gruppe über mehrere Seiten: **JA, OK**
  - Bei durchschnittlicher Anzahl Fotos/Seite oft notwendig

### 3. ~~Performance~~ ✅ GEKLÄRT
- ✅ Timeout: **CLI-Parameter, default 30s**
  - Aber: mindestens bis zur ersten Lösung warten
- ✅ Bei Timeout: **beste gefundene Lösung zurückgeben**
- ✅ **Heuristic-Fallback entfernen** - nur OR-Tools
- ✅ Erwartete Größe: **>200 Fotos**
- ⚠️ Falls zu langsam: **Iteratives Verfahren**
  - Immer nur zwei angrenzende Seiten zusammen rechnen

### 4. ~~API-Server Management~~ ✅ GEKLÄRT
- ✅ Rust startet Server **NICHT selbst**
- ✅ Annahme: Server läuft bereits (aber prüfen bei Call)
- ✅ **Kein Health-Check** - direkt /solve aufrufen

### 5. ~~Aspect-Ratio~~ ✅ GEKLÄRT
- ✅ **CLI-Parameter für max. Cropping**
- ✅ Default: **±20% OK**
- ✅ Cropping bevorzugt (kein Letterboxing)

### 6. ~~Area-Weight Implementation~~ ✅ GEKLÄRT ✨
- ✅ **Soft-Constraint mit starker Bestrafung**
- ✅ **Toleranz:** ±25% auf die Fläche akzeptabel
- ✅ **Bei Konflikten:** Klare Fehlermeldung mit Lösungsvorschlägen
- ✅ **Default-Gewichtung:** ALLE via CLI/API anpassbar
  - `weight_aspect_ratio`: 1.0
  - `weight_area`: 10.0 (hoch!)
  - `weight_group_cohesion`: 2.0
  - `weight_page_count`: 0.5

### 7. Config-Presets (offen)
- Vordefinierte Formate (Saal Digital, CEWE)?
- Config-File-Support (.toml)?

### 8. ~~Testing~~ ✅ GEKLÄRT
- ✅ **Mock-Fotos reichen aus** - einfache Rechtecke in Farbe
- ✅ **Reproduzierbare Test-Cases** erwünscht

---

## Entscheidungen getroffen ✅

### Architektur
- **Python nicht auto-installiert** - User-Verantwortung
- **uv.lock eingebacken** - Reproduzierbare Builds
- **Zwei Solver-Modi** - CLI (einfach) + API (schnell)
- **FastAPI für Modelle** - Auto-Schema, Docs
- **Typer für CLI** - Type-safe
- **Kein path in API** - Nur photo_id
- **Rust startet Server nicht** - User-Verantwortung

### Entwicklung
- **Git-Flow** - Feature-Branches + Review vor Merge
- **Conventional Commits** - feat/fix/test/docs/refactor
- **Testing** - Coverage > 90% für alle Features
- **Type Safety** - Full type annotations (Python mypy strict, Rust)
- **Linting** - ruff (Python), clippy (Rust)
- **Logging** - Gemeinsame Session-ID für Rust + Python
- **Debug-Mode** - JSON + Logs in output-Ordner

### Features
- **area_weight pro Foto** - Relative Fläche (default: 1.0)
- **max_aspect_deviation** - CLI-Parameter (default: 0.2 = 20%)
- **Timeout-Parameter** - CLI (default: 30s, mindestens erste Lösung)

### Solver
- **Nur OR-Tools** - Kein Heuristic-Fallback
- **Diskretisierung: 1mm** - AddNoOverlap2D
- **Variable Foto-Größen** - Input-Dimensionen ignorieren
- **Gruppen-Links-Constraint** - max(x_A) < min(x_B)
- **Iterativ bei >200 Fotos** - Zwei angrenzende Seiten (falls nötig)

---

## Referenzen

- **OR-Tools:** https://developers.google.com/optimization
- **CP-SAT:** https://developers.google.com/optimization/cp/cp_solver
- **uv:** https://github.com/astral-sh/uv
- **FastAPI:** https://fastapi.tiangolo.com
- **Typer:** https://typer.tiangolo.com
- **Typst:** https://typst.app

---

## Artificial Input Generator für Tests

**Ziel:** Reproduzierbare Test-Cases ohne echte Fotos

### Script: `test_photos/artificial_input_generator.py`

Vollautomatisches Generieren von Test-Daten mit Typer-CLI.

**Features:**
- ✅ **Zufällige Farben** - RGB-Werte für jedes Foto
- ✅ **Zufällige Größen** - Verschiedene Aspect-Ratios (4:3, 16:9, 1:1, etc.)
- ✅ **Gruppierung** - Fotos in lexikalisch sortierbaren Ordnern
- ✅ **Labeliert** - Foto-Info direkt im Bild sichtbar
- ✅ **Reproduzierbar** - Mit `--seed` für deterministische Ausgabe
- ✅ **Konfigurierbar** - CLI-Optionen für Gruppen & Fotos

### Usage

```bash
# Dependencies (bereits in pyproject.toml)
cd python && uv sync

# Hilfe anzeigen
python test_photos/artificial_input_generator.py --help

# Standard: 3 Gruppen mit 3-8 Fotos
python test_photos/artificial_input_generator.py generate

# Custom: 5 Gruppen mit 4-10 Fotos
python test_photos/artificial_input_generator.py generate \\
  --groups 5 \\
  --min 4 \\
  --max 10 \\
  --output my_test_data
# Reproduzierbar mit Seed
python test_photos/artificial_input_generator.py generate --seed 42
# Testen
cargo run -- --input test_photos_generated/
```

### CLI-Optionen

```
generate [OPTIONS]

Options:
  -g, --groups INTEGER   Anzahl der Gruppen [default: 3]
  --min INTEGER         Min. Fotos pro Gruppe [default: 3]
  --max INTEGER         Max. Fotos pro Gruppe [default: 8]
  -o, --output PATH     Ausgabe-Verzeichnis [default: test_photos_generated]
  -s, --seed INTEGER    Random seed für Reproduzierbarkeit [optional]
  --help                Show this message and exit
```

### Beispiel-Output

```
📸 Generiere 3 Gruppen mit 3-8 Fotos...
📁 Ausgabe: /path/to/test_photos_generated

  📂 Gruppe 1/3: 2024-01-01_Urlaub (5 Fotos)
    ✅ 5 Fotos erstellt
  📂 Gruppe 2/3: 2024-01-31_Geburtstag (7 Fotos)
    ✅ 7 Fotos erstellt
  📂 Gruppe 3/3: 2024-03-01_Wanderung (4 Fotos)
    ✅ 4 Fotos erstellt

✨ Fertig! 16 Fotos in 3 Gruppen generiert.
📂 Verzeichnis: /path/to/test_photos_generated
```

### Gruppennamen

Gruppen werden **lexikalisch sortierbar** benannt:
- Format: `YYYY-MM-DD_Thema`
- Beispiel: `2024-01-15_Urlaub`, `2024-02-20_Geburtstag`
- **Wichtig:** Sortierung = lexikalische Ordnung der Gruppennamen

### Aspect-Ratios

Zufällige Auswahl aus:
- **4:3, 3:4** - Klassisch (Landscape/Portrait)
- **16:9, 9:16** - Wide/Tall
- **1:1** - Square
- **3:2** - Classic

---

**Ende der Planung. Bereit für Implementation!** 🚀
