# Implementation Plan: `fotobuch build`

Stand: 2026-03-08

## Überblick

Inkrementeller Build: Preview-Cache erzeugen, Solver aufrufen (nur wo nötig), Typst kompilieren. Mit `--release`: Final-Cache + Final-PDF.

## Abhängigkeiten

- `dto_models::ProjectState` (vorhanden)
- `git::commit`, `git::is_git_repo` (vorhanden)
- `solver::solver::run_solver` + `solver::solver::Request` + `solver::solver::RequestType` (vorhanden)
- `image` crate (vorhanden? prüfen), `rayon` für paralleles Resizing
- `typst-cli` Binary via `Command` oder `typst` crate
- Neue Module: `cache::preview`, `cache::final_cache`

## Modulstruktur (neu anlegen)

```text
src/cache.rs          # Modul-Deklaration
src/cache/
  preview.rs          # Preview-Bilder erzeugen (resize + Wasserzeichen via Typst)
  final_cache.rs      # Final-Bilder erzeugen (300 DPI aus Original)
src/output/
  typst.rs            # typst compile aufrufen
```

Da Wasserzeichen im Typst-Template gerendert werden (nicht im Bild selbst), braucht der Preview-Cache **kein** Wasserzeichen ins Bild einzubauen.

## Ablauf: Erster Build (`layout` leer)

1. **Git pre-commit**: `pre-build: {n} photos in {g} groups` — falls bereits Commits vorhanden
2. **Preview-Cache** erzeugen: alle Fotos aus `state.photos` → `.fotobuch/cache/preview/<id>`
3. **`run_solver` MultiPage**: alle Fotos verteilen und layouten

   ```rust
   let pages = run_solver(&Request {
       request_type: RequestType::MultiPage,
       groups: &state.photos,
       config: &state.config.book_layout_solver,
       ga_config: &state.config.page_layout_solver,
       book_config: &state.config.book,
   })?;
   state.layout = pages;
   ```

4. **YAML schreiben**
5. **Typst kompilieren**: `fotobuch_preview.typ` → `fotobuch_preview.pdf`
6. **Git post-commit**: `post-build: {p} pages (cost: {total_cost})`

## Ablauf: Inkrementeller Build

1. **Änderungserkennung** (analog `status`): welche Seiten brauchen Rebuild?
2. **Preview-Cache** prüfen: fehlende/veraltete Previews (mtime-Vergleich Original vs. Cache) nacherzeugen
3. Für jede geänderte Seite: **`run_solver` SinglePage** mit den Photos dieser Seite

   ```rust
   // Synthetische PhotoGroup aus den IDs der Seite aufbauen:
   let group = PhotoGroup { files: photos_of_page, .. };
   let pages = run_solver(&Request {
       request_type: RequestType::SinglePage,
       groups: &[group],
       ..
   })?;
   state.layout[i].slots = pages[0].slots.clone();
   ```

4. Falls keine Änderungen: `Nothing to do.`
5. **YAML schreiben**, **Typst kompilieren**, **Git commit**

## Ablauf: `--release`

1. **Zustand prüfen**: Layout muss `clean` sein (kein uncommitted diff) → sonst Fehler
2. **Final-Cache** erzeugen: für jedes Foto in `layout[].photos`:
   - Slot-Größe aus `layout[i].slots[j]` → `px = mm / 25.4 * 300`
   - Immer aus Original (`source`), kein Incremental
   - Kein Upsampling: falls Original kleiner → Original verwenden + Warning
   - JPEG Qualität 95, Lanczos3 (Faktor ≤ 2) oder Triangle
3. **Typst kompilieren**: `fotobuch_final.typ` → `fotobuch_final.pdf`
4. **DPI-Validierung**: für jedes platzierte Foto `actual_dpi = px_original / (slot_mm / 25.4)` prüfen
5. **Git commit**: `release: {p} pages, {n} photos`

## Preview-Cache Implementierung

```rust
// cache/preview.rs
pub fn ensure_preview(state: &ProjectState, project_root: &Path) -> Result<()>
```

- Iteriert über alle `state.photos[].files`
- Cache-Pfad: `project_root/.fotobuch/cache/preview/<photo.id>`
- Skip wenn Cache neuer als Original (mtime-Vergleich)
- Resize: längste Kante = `config.preview.max_pixel_per_dimension` (default: 800px)
- `rayon::par_iter()` für Parallelisierung
- Kein Wasserzeichen im Bild (wird vom Typst-Template gerendert)

## Typst-Kompilierung
In commit a978ae6 gab es schonmal eine funktionierende typst-export funktionalität mit einer world. Kann auch von da genommen werden (war in output/export_typst.rs)
```rust
// output/typst.rs
pub fn compile(template_path: &Path, project_root: &Path) -> Result<()>
```

Via `Command::new("typst").args(["compile", template_file])`. Falls `typst` nicht im PATH: Fehler mit Hinweis.

## Neue Crates

```toml
image = "0.25"    # Falls noch nicht vorhanden
rayon = "1.10"
```

## Konventionen

- **Conventional Commits**: Jeder Teilschritt bekommt einen eigenen Commit (z.B. `feat: implement preview cache with rayon`, `feat: wire run_solver into build command`, `test: add integration test for incremental build`).
- **Tests vor jedem Commit**: `cargo test` muss fehlerfrei durchlaufen — kein Commit mit roten Tests.
- **Tests schreiben**: Für jeden neuen Teilschritt sind Unit-Tests (`#[cfg(test)]` inline) und Integrationstests (`tests/`) Pflicht.
- **`mod solver` unberührt**: Keine Änderungen in `src/solver/`. Der einzige Einstiegspunkt ist `solver::solver::run_solver`. Cache und Typst-Logik leben in `src/cache/` und `src/output/`, Orchestrierung in `src/commands/build.rs`.
- **Dateigröße**: `build.rs` wird schnell groß — bei >300 Zeilen in `build/` aufteilen (z.B. `build/cache.rs`, `build/diff.rs`, `build/release.rs`).

## Tests

- Erster Build auf leerem Projekt → YAML hat layout, PDF existiert
- Inkrementeller Build ohne Änderung → "Nothing to do"
- Preview-Cache wird korrekt angelegt (mtime-Check)
- `--release` schlägt fehl wenn Layout nicht clean
- DPI-Warnung wird korrekt berechnet
