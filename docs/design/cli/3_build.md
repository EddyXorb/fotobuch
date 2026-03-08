# Implementation Plan: `fotobuch build`

Stand: 2026-03-08

## Überblick

Inkrementeller Build: Preview-Cache erzeugen, Solver aufrufen (nur wo nötig), Typst kompilieren. Mit `--release`: Final-Cache + Final-PDF. Mit `--pages`: Scope auf bestimmte Seiten einschränken.

## Abhängigkeiten (vorhanden)

- `dto_models::ProjectState`, `PhotoGroup`, `PhotoFile`, `LayoutPage`, `Slot` — YAML-Datenmodell
- `solver::run_solver` + `solver::Request` + `solver::RequestType` — Solver-Einstiegspunkt
- `image = "0.25"`, `rayon = "1"` — bereits in Cargo.toml
- `typst = "0.14"`, `typst-pdf`, `typst-kit` — bereits in Cargo.toml

## Neue Dependency

```toml
git2 = "0.19"   # libgit2-Bindings — kein externes git-Binary nötig
```

Das bestehende `src/git.rs` (nutzt `Command::new("git")`) wird auf `git2` umgestellt. Damit funktioniert das Programm auch auf Systemen ohne installiertes Git.

## Neue Module

```text
src/cache.rs              # Modul-Deklaration
src/cache/
  preview.rs              # Preview-Bilder erzeugen (resize, kein Wasserzeichen)
  final_cache.rs          # Final-Bilder erzeugen (300 DPI aus Original)
  common.rs               # Geteilte Hilfsfunktionen (Pfade, Resize-Logik)
src/output/
  typst.rs                # Typst-Kompilierung via typst-crate (nicht CLI)
src/project/
  diff.rs                 # Änderungserkennung (Struct-Vergleich current vs. committed)
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

- Preview: `.fotobuch/cache/preview/Urlaub/IMG_001.jpg`
- Final: `.fotobuch/cache/final/Urlaub/IMG_001.jpg`

**Hinweis:** Die ID-Generierung mit Duplikat-Suffix (`_1`, `_2`, ...) muss im `add`-Command korrekt implementiert sein. Das ist nicht Teil dieses Plans.

---

## Ablauf: Erster Build (`layout` leer)

1. **Git pre-commit**: `pre-build: {n} photos in {g} groups`
2. **Preview-Cache** erzeugen (alle Fotos)
3. **`run_solver` MultiPage**: alle Fotos verteilen + layouten
4. **YAML schreiben** (`state.save()`)
5. **Typst kompilieren**: `fotobuch_preview.typ` → `fotobuch_preview.pdf`
6. **Git post-commit**: `post-build: {p} pages (cost: {total_cost})`

## Ablauf: Inkrementeller Build

1. **Git pre-commit**: `pre-build: pages {modified_pages} modified`
2. **Preview-Cache** prüfen: fehlende/veraltete Previews nacherzeugen (mtime-Vergleich)
3. **Änderungserkennung**: `project::diff::detect_changes()` — Struct-Diff current vs. letzter Commit
4. Für jede Seite die Rebuild braucht: **`run_solver` SinglePage**
5. Falls keine Änderungen und keine fehlenden Previews: `Nothing to do.` (kein Commit, kein PDF)
6. Falls nur Swaps (kein Rebuild nötig): PDF neu kompilieren, aber keinen Solver aufrufen
7. **YAML schreiben**, **Typst kompilieren**, **Git post-commit**

## Ablauf: `--pages <N,M,...>`

Schränkt den Scope des Builds ein:

- **Erster Build**: `--pages` wird ignoriert (es gibt kein Layout zum Einschränken)
- **Inkrementeller Build**: Nur die angegebenen Seiten werden auf Änderungen geprüft und ggf. neu gelayoutet. Andere modifizierte Seiten werden übersprungen.
- **Release**: `--pages` ist nicht erlaubt → Fehler. Release muss immer das gesamte Buch umfassen.

## Ablauf: `--release`

1. **Zustand prüfen**: Layout muss `clean` sein → sonst Fehler mit Hinweis auf `fotobuch build`
2. **DPI-Validierung + Final-Cache** erzeugen (kombiniert in einem Durchlauf):
   - Slot-Größe → `px = mm / 25.4 * 300`
   - Immer aus Original (`source`), kein Incremental
   - Kein Upsampling: falls Original kleiner → Original kopieren + DPI-Warning sammeln
   - JPEG Qualität 95, Lanczos3 (Faktor ≤ 2) oder Triangle (Faktor > 2)
3. **DPI-Warnungen ausgeben** (vor Kompilierung, damit der User sie sofort sieht)
4. **Typst kompilieren**: `fotobuch_final.typ` → `fotobuch_final.pdf`
5. **Git commit**: `release: {p} pages, {n} photos`

---

## Signaturen und Strukturen

### `src/cache/common.rs` — Geteilte Hilfsfunktionen

```rust
use std::path::{Path, PathBuf};

const PREVIEW_DIR: &str = ".fotobuch/cache/preview";
const FINAL_DIR: &str = ".fotobuch/cache/final";

/// Cache-relativer Pfad: "{group}/{local_id}.jpg"
pub fn cache_rel_path(group: &str, photo_id: &str) -> PathBuf { .. }

/// Absoluter Preview-Cache-Pfad für ein Photo.
pub fn preview_path(project_root: &Path, group: &str, photo_id: &str) -> PathBuf {
    project_root.join(PREVIEW_DIR).join(cache_rel_path(group, photo_id))
}

/// Absoluter Final-Cache-Pfad für ein Photo.
pub fn final_path(project_root: &Path, group: &str, photo_id: &str) -> PathBuf {
    project_root.join(FINAL_DIR).join(cache_rel_path(group, photo_id))
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

### `src/project/diff.rs` — Änderungserkennung

Wird von `build` UND `status` genutzt — daher in `project/`, nicht in `commands/`.

```rust
use crate::dto_models::{LayoutPage, ProjectState};

RATIO_TOLERANCE = 0.05 // TODO: make if configurable through config-section in yaml

/// Art der Änderung pro Seite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageChange {
    /// Fotos hinzugefügt/entfernt oder Ratio-Mismatch → Solver muss laufen
    NeedsRebuild,
    /// Nur Ratio-kompatible Swaps → kein Solver, nur PDF neu kompilieren
    SwapOnly,
    /// Keine Änderung
    Clean,
}

pub struct DiffResult {
    /// PageChange pro Seite (Index = Seiten-Index in layout[])
    pub pages: Vec<PageChange>,
}

/// Vergleicht current vs. committed Layout und klassifiziert Änderungen pro Seite.
///
/// Regeln:
/// - Foto-Anzahl geändert → NeedsRebuild
/// - Foto ersetzt durch anderes Ratio (>5% Abweichung) → NeedsRebuild
/// - area_weight geändert → NeedsRebuild
/// - Foto ersetzt durch kompatibles Ratio → SwapOnly
/// - Nichts geändert → Clean
pub fn detect_changes(
    current: &[LayoutPage],
    committed: &[LayoutPage],
    current_state: &ProjectState,
    committed_state: &ProjectState,
) -> DiffResult { .. }

/// Prüft ob zwei Fotos Ratio-kompatibel sind (≤5% Abweichung).
fn ratios_compatible(ratio_a: f64, ratio_b: f64) -> bool {
    (ratio_a - ratio_b).abs() / ratio_a.max(ratio_b) <= RATIO_TOLERANCE
}
```

**Committed State laden** — nutzt `git2` zum Lesen aus HEAD (siehe `src/git.rs` Signaturen weiter unten).

### `src/project/diff.rs` — Hilfsfunktion für Photo-Lookup

```rust
use crate::dto_models::{PhotoFile, ProjectState};
use std::collections::HashMap;

/// Baut einen Index photo_id → (PhotoFile, group_name) für schnellen Lookup.
pub fn build_photo_index(state: &ProjectState) -> HashMap<&str, (&PhotoFile, &str)> {
    state.photos.iter()
        .flat_map(|g| g.files.iter().map(move |f| (f.id.as_str(), (f, g.group.as_str()))))
        .collect()
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
pub fn compile_preview(project_root: &Path) -> Result<PathBuf> {
    let template = project_root.join("fotobuch_preview.typ");
    let output = project_root.join("fotobuch_preview.pdf");
    compile(&template, &output)?;
    Ok(output)
}

/// Kompiliert das Final-PDF.
pub fn compile_final(project_root: &Path) -> Result<PathBuf> {
    let template = project_root.join("fotobuch_final.typ");
    let output = project_root.join("fotobuch_final.pdf");
    compile(&template, &output)?;
    Ok(output)
}
```

**Hinweis zur TypstWorld**: Die Implementierung aus a978ae6 muss auf `typst 0.14` aktualisiert werden. Das ist der komplexeste Teil dieses Moduls — Details zur World-Implementierung werden beim Implementieren aus dem alten Commit übernommen und angepasst.

### `src/commands/build.rs` — Orchestrierung

```rust
use crate::cache::{final_cache, preview};
use crate::dto_models::ProjectState;
use crate::output::typst;
use crate::project::diff::{self, PageChange};
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
    let mut state = ProjectState::load(&project_root.join("fotobuch.yaml"))?;

    if config.release {
        if config.pages.is_some() {
            anyhow::bail!("--pages is not allowed with --release (must build entire book)");
        }
        return release_build(&state, project_root);
    }

    if state.layout.is_empty() {
        first_build(&mut state, project_root)
    } else {
        incremental_build(&mut state, project_root, config.pages.as_deref())
    }
}
```

#### Erster Build

```rust
fn first_build(state: &mut ProjectState, project_root: &Path) -> Result<BuildResult> {
    // 1. Git pre-commit
    git::commit_if_changed(project_root, &format!(
        "pre-build: {} photos in {} groups",
        state.photos.iter().map(|g| g.files.len()).sum::<usize>(),
        state.photos.len()
    ))?;

    // 2. Preview-Cache
    let progress = AtomicUsize::new(0);
    let cache_result = preview::ensure_previews(state, project_root, &progress)?;

    // 3. Solver MultiPage
    let pages = run_solver(&Request {
        request_type: RequestType::MultiPage,
        groups: &state.photos,
        config: &state.config.book_layout_solver,
        ga_config: &state.config.page_layout_solver,
        book_config: &state.config.book,
    })?;
    let total_cost = sum_page_costs(&pages); // TODO: aus Solver-Ergebnis ableiten
    state.layout = pages;

    // 4. YAML schreiben
    state.save(&project_root.join("fotobuch.yaml"))?;

    // 5. Typst kompilieren
    let pdf_path = typst::compile_preview(project_root)?;

    // 6. Git post-commit
    git::commit_if_changed(project_root, &format!(
        "post-build: {} pages (cost: {:.4})",
        state.layout.len(), total_cost
    ))?;

    Ok(BuildResult { pdf_path, pages_rebuilt: (1..=state.layout.len()).collect(), .. })
}
```

#### Inkrementeller Build

```rust
fn incremental_build(
    state: &mut ProjectState,
    project_root: &Path,
    page_filter: Option<&[usize]>,  // 1-basiert
) -> Result<BuildResult> {
    // 1. Git pre-commit (erfasst manuelle YAML-Änderungen)
    // commit ist idempotent: kein Commit wenn nichts geändert
    git::commit_if_changed(project_root, "pre-build: checking for changes")?;

    // 2. Preview-Cache
    let progress = AtomicUsize::new(0);
    preview::ensure_previews(state, project_root, &progress)?;

    // 3. Änderungserkennung
    let committed_state = load_committed_state(project_root)?;
    let diff = diff::detect_changes(
        &state.layout, &committed_state.layout, state, &committed_state
    );

    // 4. page_filter anwenden: nur angegebene Seiten berücksichtigen
    let effective_changes = apply_page_filter(&diff, page_filter);

    let pages_needing_rebuild: Vec<usize> = /* NeedsRebuild aus effective_changes */;
    let pages_swap_only: Vec<usize> = /* SwapOnly aus effective_changes */;

    if pages_needing_rebuild.is_empty() && pages_swap_only.is_empty() {
        return Ok(BuildResult { nothing_to_do: true, .. });
    }

    // 5. Solver für jede Seite die Rebuild braucht
    let photo_index = diff::build_photo_index(state);
    for &page_idx in &pages_needing_rebuild {
        rebuild_single_page(state, page_idx, &photo_index)?;
    }

    // 6. YAML + Typst + Git
    state.save(&project_root.join("fotobuch.yaml"))?;
    let pdf_path = typst::compile_preview(project_root)?;
    git::commit_if_changed(project_root, &format!("post-build: {} pages rebuilt", pages_needing_rebuild.len()))?;

    Ok(BuildResult { pages_rebuilt: pages_needing_rebuild, pages_swapped: pages_swap_only, .. })
}

/// Committed YAML laden via git show.
fn load_committed_state(project_root: &Path) -> Result<ProjectState> {
    let bytes = git::read_committed_file(project_root, "fotobuch.yaml")?
        .ok_or_else(|| anyhow::anyhow!("No committed fotobuch.yaml found"))?;
    let state: ProjectState = serde_yaml::from_slice(&bytes)?;
    Ok(state)
}

/// Wendet den --pages Filter an. None = alle Seiten.
fn apply_page_filter(diff: &DiffResult, filter: Option<&[usize]>) -> Vec<(usize, PageChange)> { .. }

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
fn release_build(state: &ProjectState, project_root: &Path) -> Result<BuildResult> {
    // 1. Clean-Check: current YAML == committed YAML
    let committed = load_committed_state(project_root)?;
    if state.layout != committed.layout {
        anyhow::bail!(
            "Layout has uncommitted changes. Run `fotobuch build` first."
        );
    }

    // 2. Final-Cache + DPI-Validierung (kombiniert)
    let progress = AtomicUsize::new(0);
    let final_result = final_cache::build_final_cache(state, project_root, &progress)?;

    // 3. DPI-Warnungen ausgeben (VOR Kompilierung)
    // (Ausgabe macht die CLI-Schicht, hier nur sammeln)

    // 4. Typst kompilieren
    let pdf_path = typst::compile_final(project_root)?;

    // 5. Git commit
    let total_photos: usize = state.layout.iter().map(|p| p.photos.len()).sum();
    git::commit_if_changed(project_root, &format!(
        "release: {} pages, {} photos", state.layout.len(), total_photos
    ))?;

    Ok(BuildResult {
        pdf_path,
        dpi_warnings: final_result.dpi_warnings,
        ..
    })
}
```

### `src/git.rs` — Komplett auf `git2` umstellen

Das bestehende `git.rs` (nutzt `std::process::Command`) wird ersetzt durch `git2`-basierte Implementierung. Kein externes Git-Binary nötig.

```rust
use git2::{Repository, Signature, IndexAddOption};
use std::path::Path;

/// Öffnet das Git-Repo im Projektverzeichnis.
fn open_repo(project_dir: &Path) -> Result<Repository> {
    Repository::open(project_dir).context("Not a git repository")
}

/// Prüft ob das Verzeichnis ein Git-Repo ist.
pub fn is_git_repo(dir: &Path) -> bool {
    Repository::open(dir).is_ok()
}

/// Initialisiert ein neues Git-Repo mit Branch "fotobuch".
pub fn init(project_dir: &Path) -> Result<Repository> {
    let repo = Repository::init(project_dir)?;
    // Initial branch auf "fotobuch" setzen
    // (git2 erstellt default "master", umbenennen nach erstem Commit)
    Ok(repo)
}

/// Staged fotobuch.yaml und committed — idempotent (kein Commit wenn nichts geändert).
/// Gibt Ok(true) zurück wenn ein Commit erstellt wurde, Ok(false) wenn nichts zu tun war.
pub fn commit_if_changed(project_dir: &Path, message: &str) -> Result<bool> {
    let repo = open_repo(project_dir)?;
    let mut index = repo.index()?;

    // fotobuch.yaml stagen
    index.add_path(Path::new("fotobuch.yaml"))?;
    index.write()?;

    // Prüfen ob sich der Index gegenüber HEAD geändert hat
    let head_tree = repo.head().ok()
        .and_then(|h| h.peel_to_tree().ok());
    let diff = repo.diff_tree_to_index(head_tree.as_ref(), Some(&index), None)?;
    if diff.deltas().count() == 0 {
        return Ok(false); // Nichts geändert
    }

    // Commit erstellen
    let oid = index.write_tree()?;
    let tree = repo.find_tree(oid)?;
    let sig = Signature::now("fotobuch", "fotobuch@local")?;
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.as_ref().map(|p| vec![p]).unwrap_or_default();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;

    Ok(true)
}

/// Liest den Inhalt einer Datei aus dem HEAD-Commit.
/// Gibt None zurück wenn kein Commit existiert oder die Datei nicht getrackt ist.
pub fn read_committed_file(project_dir: &Path, filename: &str) -> Result<Option<Vec<u8>>> {
    let repo = open_repo(project_dir)?;
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return Ok(None), // Kein Commit vorhanden
    };
    let tree = head.peel_to_tree()?;
    match tree.get_path(Path::new(filename)) {
        Ok(entry) => {
            let blob = repo.find_blob(entry.id())?;
            Ok(Some(blob.content().to_vec()))
        }
        Err(_) => Ok(None), // Datei nicht im Commit
    }
}
```

**Signature**: `fotobuch` / `fotobuch@local` als Author/Committer — die Commits sind maschinell erzeugt, kein echter User. Alternativ könnte man den globalen Git-User auslesen, aber das ist unnötig komplex für interne Tracking-Commits.

**Hinweis:** Die alte `commit()`-Funktion wird durch `commit_if_changed()` ersetzt. Alle Aufrufe im Build verwenden die idempotente Variante.

---

## Implementierungsreihenfolge

| #   | Schritt | Modul | Abhängig von |
| --- | ------- | ----- | ------------ |
| 1 | `cache_rel_path`, `is_cache_fresh`, `resize_and_save` | `cache/common.rs` | — |
| 2 | `ensure_previews` | `cache/preview.rs` | 1 |
| 3 | `git.rs` auf `git2` umstellen: `commit_if_changed`, `read_committed_file`, `init` | `git.rs` | — |
| 4 | `detect_changes`, `build_photo_index`, `ratios_compatible` | `project/diff.rs` | 3 |
| 5 | `compile` (TypstWorld aus a978ae6 portieren) | `output/typst.rs` | — |
| 6 | `first_build` (ohne `--release`, ohne `--pages`) | `commands/build.rs` | 2, 5 |
| 7 | `incremental_build` (mit `--pages`) | `commands/build.rs` | 4, 6 |
| 8 | `build_final_cache`, `target_pixels`, `actual_dpi` | `cache/final_cache.rs` | 1 |
| 9 | `release_build` | `commands/build.rs` | 7, 8 |

Jeder Schritt = ein Commit. Tests vor jedem Commit.

## Konventionen

- **Conventional Commits**: z.B. `feat: implement preview cache with rayon`, `feat(build): wire solver into first build`
- **Tests**: Unit-Tests (`#[cfg(test)]` inline) + Integrationstests (`tests/`) für jeden Schritt
- **`mod solver` unberührt**: Keine Änderungen in `src/solver/`. Einziger Einstiegspunkt: `solver::run_solver`
- **Dateigröße**: `build.rs` bei >300 Zeilen in Submodule aufteilen (z.B. `build/first.rs`, `build/incremental.rs`, `build/release.rs`)
- **Eine neue Crate**: `git2 = "0.19"` (libgit2-Bindings). `image`, `rayon`, `typst` sind bereits in Cargo.toml

## Tests

| Test | Prüft |
| ---- | ----- |
| Preview-Cache: frisches Bild wird erzeugt | `ensure_previews` mit leerem Cache |
| Preview-Cache: unverändertes Bild wird übersprungen | mtime-Check Logik |
| `cache_rel_path` Ableitung | Prefix-Stripping, Suffix-Handling |
| `detect_changes`: Foto-Swap gleicher Ratio → SwapOnly | Ratio-Toleranz |
| `detect_changes`: Foto hinzugefügt → NeedsRebuild | Längenänderung photos[] |
| `detect_changes`: area_weight geändert → NeedsRebuild | Weight-Vergleich |
| Erster Build auf leerem Projekt → YAML hat layout, PDF existiert | End-to-End |
| Inkrementeller Build ohne Änderung → `nothing_to_do: true` | Idempotenz |
| `--release` schlägt fehl wenn Layout nicht clean | Clean-Check |
| DPI-Warnung: Original kleiner als Slot → Warning mit korrektem DPI-Wert | `actual_dpi` |
| `--pages` filtert korrekt | `apply_page_filter` |
| `--release --pages` → Fehler | Validierung |
