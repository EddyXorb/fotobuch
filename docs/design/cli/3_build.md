# Implementation Plan: `fotobuch build`

Stand: 2026-03-08

## Überblick

Inkrementeller Build: Preview-Cache erzeugen, Solver aufrufen (nur wo nötig), Typst kompilieren. Mit `--release`: Final-Cache + Final-PDF. Mit `--pages`: Scope auf bestimmte Seiten einschränken.

## Abhängigkeiten (vorhanden)

- `dto_models::ProjectState`, `PhotoGroup`, `PhotoFile`, `LayoutPage`, `Slot` — YAML-Datenmodell
- `solver::run_solver` + `solver::Request` + `solver::RequestType` — Solver-Einstiegspunkt
- `image = "0.25"`, `rayon = "1"` — bereits in Cargo.toml
- `typst = "0.14"`, `typst-pdf`, `typst-kit` — bereits in Cargo.toml

## Neue Module

```text
src/cache.rs              # Modul-Deklaration
src/cache/
  preview.rs              # Preview-Bilder erzeugen (resize, kein Wasserzeichen)
  final_cache.rs          # Final-Bilder erzeugen (300 DPI aus Original)
  common.rs               # Geteilte Hilfsfunktionen (Pfade, Resize-Logik)
src/output/
  typst.rs                # Typst-Kompilierung via typst-crate (nicht CLI)
```

Wasserzeichen werden **nicht** ins Bild eingebaut — das Typst-Preview-Template rendert sie als `#place()` + `#rotate()` + `#text()` Overlay. Das Template ist eine eigene Aufgabe (nicht Teil dieses Plans).

## Cache-Pfade

Format: `{group}/{local_id}.jpg`

`local_id` wird aus `photo.id` abgeleitet durch Entfernen des Gruppen-Prefix:

```rust
// cache/common.rs
/// Leitet den relativen Cache-Pfad aus Gruppe und Photo-ID ab.
/// Bsp: group="Urlaub", id="Urlaub_IMG_001" → "Urlaub/IMG_001.jpg"
///      group="Urlaub", id="Urlaub_IMG_001_1" → "Urlaub/IMG_001_1.jpg"
pub fn cache_rel_path(group: &str, photo_id: &str) -> PathBuf {
    let local_id = photo_id
        .strip_prefix(group)
        .and_then(|s| s.strip_prefix('_'))
        .unwrap_or(photo_id);
    PathBuf::from(group).join(format!("{local_id}.jpg"))
}
```

Vollständige Pfade:

- Preview: `.fotobuch/cache/{projektname}/preview/Urlaub/IMG_001.jpg`
- Final: `.fotobuch/cache/{projektname}/final/Urlaub/IMG_001.jpg`

**Hinweis:** Die ID-Generierung mit Duplikat-Suffix (`_1`, `_2`, ...) muss im `add`-Command korrekt implementiert sein. Das ist nicht Teil dieses Plans.

---

## Ablauf: Erster Build (`layout` leer)

1. **StateManager::open()** — erfasst automatisch Nutzer-Edits (kein expliziter pre-commit nötig)
2. **Preview-Cache** erzeugen (alle Fotos)
3. **`run_solver` MultiPage**: alle Fotos verteilen + layouten
4. **Typst kompilieren**: `{name}.typ` → `{name}.pdf`
5. **`mgr.finish("build: ...")`** — schreibt YAML und committet

## Ablauf: Inkrementeller Build

1. **StateManager::open()** — erfasst automatisch Nutzer-Edits (User-Edit-Commit, idempotent)
2. **Preview-Cache** prüfen: fehlende/veraltete Previews nacherzeugen (mtime-Vergleich)
3. **Änderungserkennung**: `StateManager::StateDiff` — Struct-Diff current vs. committed (intern in StateManager)
4. Für jede Seite die Rebuild braucht: **`run_solver` SinglePage**
5. Falls keine Änderungen und keine fehlenden Previews: `Nothing to do.` (kein Commit, kein PDF)
6. Falls nur Swaps (kein Rebuild nötig): PDF neu kompilieren, aber keinen Solver aufrufen
7. **`mgr.finish("build: ...")`** — schreibt YAML und committet

## Ablauf: `--pages <N,M,...>`

Schränkt den Scope des Builds ein:

- **Erster Build**: `--pages` wird ignoriert (es gibt kein Layout zum Einschränken)
- **Inkrementeller Build**: Nur die angegebenen Seiten werden auf Änderungen geprüft und ggf. neu gelayoutet. Andere modifizierte Seiten werden übersprungen.
- **Release**: `--pages` ist nicht erlaubt → Fehler. Release muss immer das gesamte Buch umfassen.

## Ablauf: `--release`

1. **StateManager::open()** — wie immer; Release setzt voraus dass Layout clean ist
2. **Zustand prüfen**: Layout muss `clean` sein → sonst Fehler mit Hinweis auf `fotobuch build`
3. **DPI-Validierung + Final-Cache** erzeugen (kombiniert in einem Durchlauf):
   - Slot-Größe → `px = mm / 25.4 * 300`
   - Immer aus Original (`source`), kein Incremental
   - Kein Upsampling: falls Original kleiner → Original kopieren + DPI-Warning sammeln
   - JPEG Qualität 95, Lanczos3 (Faktor ≤ 2) oder Triangle (Faktor > 2)
4. **DPI-Warnungen ausgeben** (vor Kompilierung, damit der User sie sofort sieht)
5. **Typst kompilieren**: `final.typ` (generiert aus `{name}.typ` mit `is_final = true`) → `final.pdf`
6. **`mgr.finish("release: {p} pages, {n} photos")`**

---

## Signaturen und Strukturen

### `src/cache/common.rs` — Geteilte Hilfsfunktionen

Die Cache-Pfade enthalten jetzt den Projektnamen. Bevorzugte Variante: `StateManager::preview_cache_dir()` / `StateManager::final_cache_dir()` verwenden statt eigener Konstanten.

```rust
use std::path::{Path, PathBuf};

/// Cache-relativer Pfad: "{group}/{local_id}.jpg"
pub fn cache_rel_path(group: &str, photo_id: &str) -> PathBuf { .. }

/// Absoluter Preview-Cache-Pfad für ein Photo.
/// `cache_base` kommt von `mgr.preview_cache_dir()`.
pub fn preview_path(cache_base: &Path, group: &str, photo_id: &str) -> PathBuf {
    cache_base.join(cache_rel_path(group, photo_id))
}

/// Absoluter Final-Cache-Pfad für ein Photo.
/// `cache_base` kommt von `mgr.final_cache_dir()`.
pub fn final_path(cache_base: &Path, group: &str, photo_id: &str) -> PathBuf {
    cache_base.join(cache_rel_path(group, photo_id))
}

/// true wenn `cached` existiert und neuer ist als `source`.
pub fn is_cache_fresh(source: &Path, cached: &Path) -> bool { .. }

/// Resize mit automatischer Filterwahl (Lanczos3 ≤ 2x, Triangle > 2x).
/// Speichert als JPEG mit gegebener Qualität. Erstellt Elternverzeichnisse.
pub fn resize_and_save(
    source: &Path,
    target: &Path,
    target_width: u32,
    target_height: u32,
    jpeg_quality: u8,
) -> Result<()> { .. }
```

### `src/cache/preview.rs` — Preview-Cache

```rust
use crate::dto_models::ProjectState;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct PreviewCacheResult {
    pub created: usize,
    pub skipped: usize,
    pub total: usize,
}

/// Erzeugt fehlende/veraltete Preview-Bilder für alle Fotos im Projekt.
/// Resize: längste Kante = config.preview.max_preview_px (default 800).
/// Parallelisiert via rayon. Fortschritt via AtomicUsize-Counter.
pub fn ensure_previews(
    state: &ProjectState,
    project_root: &Path,
    progress: &AtomicUsize,  // wird bei jedem fertigen Bild inkrementiert
) -> Result<PreviewCacheResult> {
    let max_px = state.config.preview.max_preview_px;

    // Alle (group, photo) Paare sammeln
    let all_photos: Vec<(&str, &PhotoFile)> = state.photos.iter()
        .flat_map(|g| g.files.iter().map(move |f| (g.group.as_str(), f)))
        .collect();

    all_photos.par_iter().try_for_each(|(group, photo)| {
        let source = Path::new(&photo.source);
        let cached = preview_path(project_root, group, &photo.id);
        if is_cache_fresh(source, &cached) {
            // skip
        } else {
            let (tw, th) = fit_dimensions(photo.width_px, photo.height_px, max_px);
            resize_and_save(source, &cached, tw, th, 85)?;
        }
        progress.fetch_add(1, Ordering::Relaxed);
        Ok(())
    })
}

/// Berechnet Zieldimensionen so dass die längste Kante = max_px.
fn fit_dimensions(width: u32, height: u32, max_px: u32) -> (u32, u32) { .. }
```

### `src/cache/final_cache.rs` — Final-Cache (300 DPI)

```rust
use crate::commands::build::DpiWarning;
use crate::dto_models::{LayoutPage, ProjectState, Slot};
use std::path::Path;
use std::sync::atomic::AtomicUsize;

pub struct FinalCacheResult {
    pub created: usize,
    pub dpi_warnings: Vec<DpiWarning>,
}

/// Erzeugt Final-Bilder für alle Fotos im Layout.
/// Immer aus Original, kein Incremental. Sammelt DPI-Warnungen.
pub fn build_final_cache(
    state: &ProjectState,
    project_root: &Path,
    progress: &AtomicUsize,
) -> Result<FinalCacheResult> { .. }

/// Berechnet Ziel-Pixel aus Slot-mm und DPI.
fn target_pixels(slot: &Slot, dpi: f64) -> (u32, u32) {
    let w = (slot.width_mm / 25.4 * dpi).round() as u32;
    let h = (slot.height_mm / 25.4 * dpi).round() as u32;
    (w, h)
}

/// Berechnet die tatsächliche DPI eines Fotos in einem Slot.
fn actual_dpi(photo_width_px: u32, photo_height_px: u32, slot: &Slot) -> f64 {
    let dpi_w = photo_width_px as f64 / (slot.width_mm / 25.4);
    let dpi_h = photo_height_px as f64 / (slot.height_mm / 25.4);
    dpi_w.min(dpi_h) // limitierender Faktor
}
```

### `src/output/typst.rs` — Typst-Kompilierung via Crate

Nutzt die vorhandenen Dependencies `typst`, `typst-pdf`, `typst-kit`. Orientiert sich an der Implementierung aus Commit a978ae6.

```rust
use std::path::{Path, PathBuf};

/// Kompiliert ein Typst-Template zu PDF.
/// Nutzt die typst-crate direkt (keine externe Binary nötig).
///
/// Implementierung: TypstWorld aufbauen (wie in a978ae6),
/// typst::compile() aufrufen, typst_pdf::pdf() für Export.
pub fn compile(template_path: &Path, output_path: &Path) -> Result<()> { .. }

/// Kompiliert das Preview-PDF.
/// `project_name` kommt aus dem StateManager (z.B. "meinbuch").
/// Template: `{project_root}/{name}.typ` → Output: `{project_root}/{name}.pdf`
pub fn compile_preview(project_root: &Path, project_name: &str) -> Result<PathBuf> {
    let template = project_root.join(format!("{project_name}.typ"));
    let output = project_root.join(format!("{project_name}.pdf"));
    compile(&template, &output)?;
    Ok(output)
}

/// Kompiliert das Final-PDF.
/// Kopiert `{name}.typ` nach `final.typ`, setzt `is_final = true`, kompiliert zu `final.pdf`.
/// Die generierte `final.typ` ist eine temporäre Datei — nicht ins Repo committen.
pub fn compile_final(project_root: &Path, project_name: &str) -> Result<PathBuf> {
    let source_template = project_root.join(format!("{project_name}.typ"));
    let final_template = project_root.join("final.typ");
    let output = project_root.join("final.pdf");
    // Vorlage kopieren und is_final = true setzen
    generate_final_template(&source_template, &final_template)?;
    compile(&final_template, &output)?;
    Ok(output)
}

/// Erzeugt `final.typ` aus dem Preview-Template mit `is_final = true`.
fn generate_final_template(source: &Path, target: &Path) -> Result<()> { .. }
```

**Hinweis zur TypstWorld**: Die Implementierung aus a978ae6 muss auf `typst 0.14` aktualisiert werden. Das ist der komplexeste Teil dieses Moduls — Details zur World-Implementierung werden beim Implementieren aus dem alten Commit übernommen und angepasst.

### `src/commands/build.rs` — Orchestrierung

```rust
use crate::cache::{final_cache, preview};
use crate::output::typst;
use crate::state_manager::StateManager;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

// --- Bestehende Structs (bereits definiert, Anpassungen) ---

#[derive(Debug)]
pub struct BuildConfig {
    pub release: bool,
    pub pages: Option<Vec<usize>>,  // 1-basiert
}

#[derive(Debug)]
pub struct BuildResult {
    pub pdf_path: PathBuf,
    pub pages_rebuilt: Vec<usize>,   // 1-basiert
    pub pages_swapped: Vec<usize>,   // 1-basiert, nur PDF-Neukompilierung
    pub total_cost: f64,
    pub dpi_warnings: Vec<DpiWarning>,
    pub preview_cache: Option<preview::PreviewCacheResult>,
    pub nothing_to_do: bool,
}

/// Haupteinstiegspunkt — dispatcht an first_build, incremental_build oder release_build.
pub fn build(project_root: &Path, config: &BuildConfig) -> Result<BuildResult> {
    let mut mgr = StateManager::open(project_root)?;

    if config.release {
        if config.pages.is_some() {
            anyhow::bail!("--pages is not allowed with --release (must build entire book)");
        }
        return release_build(&mut mgr, project_root);
    }

    if mgr.state.layout.is_empty() {
        first_build(&mut mgr, project_root)
    } else {
        incremental_build(&mut mgr, project_root, config.pages.as_deref())
    }
}
```

#### Erster Build

```rust
fn first_build(mgr: &mut StateManager, project_root: &Path) -> Result<BuildResult> {
    // 1. StateManager::open() hat bereits User-Edits erfasst — kein pre-commit nötig

    // 2. Preview-Cache
    let progress = AtomicUsize::new(0);
    let cache_result = preview::ensure_previews(&mgr.state, project_root, &progress)?;

    // 3. Solver MultiPage
    let pages = run_solver(&Request {
        request_type: RequestType::MultiPage,
        groups: &mgr.state.photos,
        config: &mgr.state.config.book_layout_solver,
        ga_config: &mgr.state.config.page_layout_solver,
        book_config: &mgr.state.config.book,
    })?;
    let total_cost = sum_page_costs(&pages); // TODO: aus Solver-Ergebnis ableiten
    mgr.state.layout = pages;

    // 4. Typst kompilieren
    let pdf_path = typst::compile_preview(project_root, &mgr.project_name())?;

    // 5. YAML schreiben + Git commit
    mgr.finish(&format!(
        "build: {} pages (cost: {:.4})",
        mgr.state.layout.len(), total_cost
    ))?;

    Ok(BuildResult { pdf_path, pages_rebuilt: (1..=mgr.state.layout.len()).collect(), .. })
}
```

#### Inkrementeller Build

```rust
fn incremental_build(
    mgr: &mut StateManager,
    project_root: &Path,
    page_filter: Option<&[usize]>,  // 1-basiert
) -> Result<BuildResult> {
    // 1. StateManager::open() hat bereits User-Edits erfasst (idempotent)

    // 2. Preview-Cache
    let progress = AtomicUsize::new(0);
    preview::ensure_previews(&mgr.state, project_root, &progress)?;

    // 3. Änderungserkennung via StateManager::StateDiff (intern)
    let diff = mgr.diff()?;

    // 4. page_filter anwenden: nur angegebene Seiten berücksichtigen
    let effective_changes = apply_page_filter(&diff, page_filter);

    let pages_needing_rebuild: Vec<usize> = /* NeedsRebuild aus effective_changes */;
    let pages_swap_only: Vec<usize> = /* SwapOnly aus effective_changes */;

    if pages_needing_rebuild.is_empty() && pages_swap_only.is_empty() {
        return Ok(BuildResult { nothing_to_do: true, .. });
    }

    // 5. Solver für jede Seite die Rebuild braucht
    let photo_index = build_photo_index(&mgr.state);
    for &page_idx in &pages_needing_rebuild {
        rebuild_single_page(&mut mgr.state, page_idx, &photo_index)?;
    }

    // 6. Typst kompilieren
    let pdf_path = typst::compile_preview(project_root, &mgr.project_name())?;

    // 7. YAML schreiben + Git commit
    mgr.finish(&format!("build: {} pages rebuilt", pages_needing_rebuild.len()))?;

    Ok(BuildResult { pages_rebuilt: pages_needing_rebuild, pages_swapped: pages_swap_only, .. })
}

/// Wendet den --pages Filter an. None = alle Seiten.
fn apply_page_filter(diff: &StateDiff, filter: Option<&[usize]>) -> Vec<(usize, PageChange)> { .. }

/// Einzelne Seite neu layouten via SinglePage-Solver.
fn rebuild_single_page(
    state: &mut ProjectState,
    page_idx: usize,  // 0-basiert
    photo_index: &HashMap<&str, (&PhotoFile, &str)>,
) -> Result<f64> {
    // PhotoGroup aus den IDs der Seite zusammenbauen
    let page = &state.layout[page_idx];
    let files: Vec<PhotoFile> = page.photos.iter()
        .filter_map(|id| photo_index.get(id.as_str()))
        .map(|(pf, _)| (*pf).clone())
        .collect();

    let group = PhotoGroup {
        group: format!("page_{}", page_idx + 1),
        sort_key: String::new(),
        files,
    };

    let result = run_solver(&Request {
        request_type: RequestType::SinglePage,
        groups: &[group],
        config: &state.config.book_layout_solver,
        ga_config: &state.config.page_layout_solver,
        book_config: &state.config.book,
    })?;

    // Nur slots übernehmen, photos-Liste bleibt unverändert
    state.layout[page_idx].slots = result[0].slots.clone();
    Ok(0.0) // TODO: cost aus Solver-Ergebnis
}
```

#### Release Build

```rust
fn release_build(mgr: &mut StateManager, project_root: &Path) -> Result<BuildResult> {
    // 1. Clean-Check: StateManager prüft ob Layout seit letztem Commit unverändert
    if !mgr.is_clean()? {
        anyhow::bail!(
            "Layout has uncommitted changes. Run `fotobuch build` first."
        );
    }

    // 2. Final-Cache + DPI-Validierung (kombiniert)
    let progress = AtomicUsize::new(0);
    let final_result = final_cache::build_final_cache(&mgr.state, project_root, &progress)?;

    // 3. DPI-Warnungen ausgeben (VOR Kompilierung)
    // (Ausgabe macht die CLI-Schicht, hier nur sammeln)

    // 4. Typst kompilieren: generiert final.typ aus {name}.typ mit is_final = true
    let pdf_path = typst::compile_final(project_root, &mgr.project_name())?;

    // 5. YAML schreiben + Git commit
    let total_photos: usize = mgr.state.layout.iter().map(|p| p.photos.len()).sum();
    mgr.finish(&format!(
        "release: {} pages, {} photos", mgr.state.layout.len(), total_photos
    ))?;

    Ok(BuildResult {
        pdf_path,
        dpi_warnings: final_result.dpi_warnings,
        ..
    })
}
```

---

## Implementierungsreihenfolge

| #   | Schritt | Modul | Abhängig von |
| --- | ------- | ----- | ------------ |
| 1 | `cache_rel_path`, `is_cache_fresh`, `resize_and_save` | `cache/common.rs` | — |
| 2 | `ensure_previews` | `cache/preview.rs` | 1 |
| 3 | `compile`, `compile_preview`, `compile_final`, `generate_final_template` | `output/typst.rs` | — |
| 4 | `first_build` (ohne `--release`, ohne `--pages`) | `commands/build.rs` | 2, 3 |
| 5 | `incremental_build` (mit `--pages`) | `commands/build.rs` | 4 |
| 6 | `build_final_cache`, `target_pixels`, `actual_dpi` | `cache/final_cache.rs` | 1 |
| 7 | `release_build` | `commands/build.rs` | 5, 6 |

Jeder Schritt = ein Commit. Tests vor jedem Commit.

## Konventionen

- **Conventional Commits**: z.B. `feat: implement preview cache with rayon`, `feat(build): wire solver into first build`
- **Tests**: Unit-Tests (`#[cfg(test)]` inline) + Integrationstests (`tests/`) für jeden Schritt
- **`mod solver` unberührt**: Keine Änderungen in `src/solver/`. Einziger Einstiegspunkt: `solver::run_solver`
- **Dateigröße**: `build.rs` bei >300 Zeilen in Submodule aufteilen (z.B. `build/first.rs`, `build/incremental.rs`, `build/release.rs`)
- **Keine neuen Crates**: `image`, `rayon`, `typst` sind bereits in Cargo.toml. Git-Operationen sind intern im StateManager.

## Tests

| Test | Prüft |
| ---- | ----- |
| Preview-Cache: frisches Bild wird erzeugt | `ensure_previews` mit leerem Cache |
| Preview-Cache: unverändertes Bild wird übersprungen | mtime-Check Logik |
| `cache_rel_path` Ableitung | Prefix-Stripping, Suffix-Handling |
| Erster Build auf leerem Projekt → YAML hat layout, PDF existiert | End-to-End |
| Inkrementeller Build ohne Änderung → `nothing_to_do: true` | Idempotenz |
| `--release` schlägt fehl wenn Layout nicht clean | `mgr.is_clean()` |
| DPI-Warnung: Original kleiner als Slot → Warning mit korrektem DPI-Wert | `actual_dpi` |
| `--pages` filtert korrekt | `apply_page_filter` |
| `--release --pages` → Fehler | Validierung |
| `compile_final` erzeugt `final.typ` mit `is_final = true` | `generate_final_template` |
