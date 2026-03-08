# Implementation Plan: `fotobuch rebuild`

Stand: 2026-03-08

## Überblick

Erzwingt Neuberechnung — mächtiger als `build`. Drei Modi: Einzelseite (nur Page-Layout-Solver), Seitenbereich (Book-Layout-Solver auf Teilmenge + Page-Layout), kompletter Neustart (alles von vorn).

## Abhängigkeiten

- `cache::preview::ensure_previews` — aus Build-Plan, Preview-Cache erzeugen
- `output::typst::compile_preview` — aus Build-Plan, PDF kompilieren
- `solver::run_solver` + `Request` + `RequestType` — Solver-Einstiegspunkt
- `git::commit_if_changed` — aus Build-Plan, `git2`-basiert
- `project::diff::build_photo_index` — aus Build-Plan, Photo-Lookup

**Keine neuen Crates.** Alles was `rebuild` braucht, wird bereits durch den Build-Plan eingeführt.

## Abgrenzung zu `build`

| Aspekt | `build` | `rebuild` |
| ------ | ------- | --------- |
| Book-Layout-Solver | Nur beim allerersten Aufruf | Bei Range oder All |
| Page-Layout-Solver | Nur für geänderte Seiten | Erzwungen für angegebene Seiten |
| Inkrementell | Ja (Änderungserkennung) | Nein (immer erzwungen) |
| Sicher | Ja | Nur bei Einzelseite |

## Wiederverwendung aus `build`

`rebuild` nutzt direkt:

- `cache::preview::ensure_previews` — Preview-Cache
- `output::typst::compile_preview` — PDF-Kompilierung
- `commands::shared::rebuild_single_page` — SinglePage-Solver (von build und rebuild genutzt)
- `project::diff::build_photo_index` — Photo-ID → PhotoFile Lookup

Kein `ensure_cache_and_compile`-Wrapper nötig — die Module werden direkt aufgerufen.

---

## Ablauf je Scope

### `RebuildScope::SinglePage(n)` — n ist 1-basiert

1. **Git pre-commit**: `pre-rebuild: page {n}`
2. **Preview-Cache** prüfen
3. **`run_solver` SinglePage** erzwungen — wiederverwendet `shared::rebuild_single_page`
4. **YAML schreiben**, **Typst kompilieren**
5. **Git post-commit**: `post-rebuild: page {n} (cost: {cost:.4})`

### `RebuildScope::Range { start, end, flex }` — start/end sind 1-basiert

1. **Git pre-commit**: `pre-rebuild: pages {start}-{end}`
2. **Preview-Cache** prüfen
3. **Fotos aus dem Bereich als PhotoGroups rekonstruieren** (siehe `collect_photos_as_groups`)
4. **`run_solver` MultiPage** mit angepassten Seitengrenzen:

   ```rust
   let n = end - start + 1;
   let config = BookLayoutSolverConfig {
       page_min: n.saturating_sub(flex).max(1),
       page_max: n + flex,
       page_target: n,
       ..state.config.book_layout_solver.clone()
   };

   let groups = collect_photos_as_groups(state, start - 1, end);
   let new_pages = run_solver(&Request {
       request_type: RequestType::MultiPage,
       groups: &groups,
       config: &config,
       ga_config: &state.config.page_layout_solver,
       book_config: &state.config.book,
   })?;

   // Bereich ersetzen (splice: kann bei flex mehr/weniger Seiten ergeben)
   state.layout.splice((start - 1)..end, new_pages);
   renumber_pages(&mut state.layout);
   ```

5. **YAML schreiben**, **Typst kompilieren**
6. **Git post-commit**: `post-rebuild: pages {start}-{end} (cost: {total_cost:.4})`

### `RebuildScope::All`

1. **Git pre-commit**: `pre-rebuild: all`
2. **Preview-Cache** prüfen
3. **`run_solver` MultiPage** auf alle Photos (inkl. bisher unplaced):

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

4. **YAML schreiben**, **Typst kompilieren**
5. **Git post-commit**: `post-rebuild: {p} pages (cost: {total_cost:.4})`

**Hinweis zu `--flex`**: Wird nur bei Range berücksichtigt. Bei SinglePage und All ignoriert (SinglePage hat fixe Fotozahl, All bestimmt Seitenzahl frei via Config-Defaults).

---

## Signaturen und Strukturen

### `src/commands/rebuild.rs` — Orchestrierung

```rust
use crate::cache::preview;
use crate::dto_models::ProjectState;
use crate::output::typst;
use crate::project::diff;
use super::build::BuildResult;
use super::shared;
use std::path::Path;
use std::sync::atomic::AtomicUsize;

/// Scope bleibt wie im bestehenden Stub, keine Änderung nötig.
#[derive(Debug, Clone)]
pub enum RebuildScope {
    All,
    SinglePage(usize),                              // 1-basiert
    Range { start: usize, end: usize, flex: usize }, // 1-basiert
}

/// Haupteinstiegspunkt — dispatcht an die drei Modi.
pub fn rebuild(project_root: &Path, scope: RebuildScope) -> Result<BuildResult> {
    let mut state = ProjectState::load(&project_root.join("fotobuch.yaml"))?;

    // Validierung: Layout muss existieren (außer bei All)
    if !matches!(scope, RebuildScope::All) && state.layout.is_empty() {
        anyhow::bail!(
            "No layout exists. Run `fotobuch build` first, \
             or use `fotobuch rebuild` (without arguments) for a full rebuild."
        );
    }

    // Scope-Validierung
    if let RebuildScope::Range { start, end, .. } = &scope {
        if *start == 0 || *end == 0 || *start > *end || *end > state.layout.len() {
            anyhow::bail!(
                "Invalid page range {}-{} (layout has {} pages)",
                start, end, state.layout.len()
            );
        }
    }
    if let RebuildScope::SinglePage(n) = &scope {
        if *n == 0 || *n > state.layout.len() {
            anyhow::bail!(
                "Invalid page {} (layout has {} pages)",
                n, state.layout.len()
            );
        }
    }

    match scope {
        RebuildScope::SinglePage(n) => rebuild_single(project_root, &mut state, n),
        RebuildScope::Range { start, end, flex } => {
            rebuild_range(project_root, &mut state, start, end, flex)
        }
        RebuildScope::All => rebuild_all(project_root, &mut state),
    }
}
```

#### Einzelseite

```rust
fn rebuild_single(
    project_root: &Path,
    state: &mut ProjectState,
    page: usize,  // 1-basiert
) -> Result<BuildResult> {
    // 1. Pre-commit
    git::commit_if_changed(project_root, &format!("pre-rebuild: page {page}"))?;

    // 2. Preview-Cache
    let progress = AtomicUsize::new(0);
    preview::ensure_previews(state, project_root, &progress)?;

    // 3. Solver — wiederverwendet shared::rebuild_single_page
    let photo_index = diff::build_photo_index(state);
    let cost = shared::rebuild_single_page(state, page - 1, &photo_index)?;

    // 4. YAML + Typst
    state.save(&project_root.join("fotobuch.yaml"))?;
    let pdf_path = typst::compile_preview(project_root)?;

    // 5. Post-commit
    git::commit_if_changed(project_root, &format!(
        "post-rebuild: page {page} (cost: {cost:.4})"
    ))?;

    Ok(BuildResult {
        pdf_path,
        pages_rebuilt: vec![page],
        ..Default::default()
    })
}
```

#### Seitenbereich

```rust
fn rebuild_range(
    project_root: &Path,
    state: &mut ProjectState,
    start: usize,  // 1-basiert
    end: usize,    // 1-basiert
    flex: usize,
) -> Result<BuildResult> {
    // 1. Pre-commit
    git::commit_if_changed(project_root, &format!("pre-rebuild: pages {start}-{end}"))?;

    // 2. Preview-Cache
    let progress = AtomicUsize::new(0);
    preview::ensure_previews(state, project_root, &progress)?;

    // 3. Fotos aus Bereich als PhotoGroups rekonstruieren
    let groups = collect_photos_as_groups(state, start - 1, end);

    // 4. Solver mit angepassten Seitengrenzen
    let n = end - start + 1;
    let config = BookLayoutSolverConfig {
        page_min: n.saturating_sub(flex).max(1),
        page_max: n + flex,
        page_target: n,
        ..state.config.book_layout_solver.clone()
    };

    let new_pages = run_solver(&Request {
        request_type: RequestType::MultiPage,
        groups: &groups,
        config: &config,
        ga_config: &state.config.page_layout_solver,
        book_config: &state.config.book,
    })?;

    let pages_rebuilt: Vec<usize> = (start..start + new_pages.len()).collect();
    let total_cost = 0.0; // TODO: aus Solver-Ergebnis

    // 5. Layout aktualisieren + renumbern
    state.layout.splice((start - 1)..end, new_pages);
    renumber_pages(&mut state.layout);

    // 6. YAML + Typst
    state.save(&project_root.join("fotobuch.yaml"))?;
    let pdf_path = typst::compile_preview(project_root)?;

    // 7. Post-commit
    git::commit_if_changed(project_root, &format!(
        "post-rebuild: pages {start}-{end} (cost: {total_cost:.4})"
    ))?;

    Ok(BuildResult {
        pdf_path,
        pages_rebuilt,
        ..Default::default()
    })
}
```

#### Kompletter Neustart

```rust
fn rebuild_all(
    project_root: &Path,
    state: &mut ProjectState,
) -> Result<BuildResult> {
    // 1. Pre-commit
    git::commit_if_changed(project_root, "pre-rebuild: all")?;

    // 2. Preview-Cache
    let progress = AtomicUsize::new(0);
    preview::ensure_previews(state, project_root, &progress)?;

    // 3. Solver MultiPage auf alle Photos (inkl. unplaced)
    let pages = run_solver(&Request {
        request_type: RequestType::MultiPage,
        groups: &state.photos,
        config: &state.config.book_layout_solver,
        ga_config: &state.config.page_layout_solver,
        book_config: &state.config.book,
    })?;

    let pages_rebuilt: Vec<usize> = (1..=pages.len()).collect();
    let total_cost = 0.0; // TODO: aus Solver-Ergebnis
    state.layout = pages;

    // 4. YAML + Typst
    state.save(&project_root.join("fotobuch.yaml"))?;
    let pdf_path = typst::compile_preview(project_root)?;

    // 5. Post-commit
    git::commit_if_changed(project_root, &format!(
        "post-rebuild: {} pages (cost: {total_cost:.4})", state.layout.len()
    ))?;

    Ok(BuildResult {
        pdf_path,
        pages_rebuilt,
        ..Default::default()
    })
}
```

### `src/commands/shared.rs` — Geteilte Logik zwischen build und rebuild

Enthält Funktionen die sowohl von `build` als auch `rebuild` genutzt werden.

```rust
use crate::dto_models::{PhotoFile, PhotoGroup, ProjectState};
use crate::solver::{run_solver, Request, RequestType};
use std::collections::HashMap;

/// Einzelne Seite neu layouten via SinglePage-Solver.
/// Wird von build (inkrementell) und rebuild (SinglePage) verwendet.
///
/// page_idx: 0-basiert
/// Gibt den Cost des Solver-Ergebnisses zurück.
pub fn rebuild_single_page(
    state: &mut ProjectState,
    page_idx: usize,
    photo_index: &HashMap<&str, (&PhotoFile, &str)>,
) -> Result<f64> {
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

### Hilfsfunktionen in `src/commands/rebuild.rs`

#### Gruppenrekonstruktion für Range-Rebuild

Die Photos aus den Seiten im Bereich müssen zurück in ihre **ursprünglichen Gruppen** sortiert werden, damit der MIP-Solver die Gruppen-Constraints korrekt anwenden kann (`group_max_per_page`, `weight_split`).

```rust
use crate::project::diff::build_photo_index;

/// Sammelt alle Fotos aus dem Seitenbereich und rekonstruiert PhotoGroups.
///
/// start: 0-basiert (inclusive)
/// end: 1-basiert (= exklusiv, passt zu layout[start..end] und splice)
///
/// Ablauf:
/// 1. Alle photo_ids aus layout[start..end].photos sammeln
/// 2. Jede ID via photo_index → (PhotoFile, group_name) auflösen
/// 3. Nach group_name gruppieren, sort_key aus der Originalgruppe übernehmen
/// 4. PhotoGroups zurückgeben, sortiert nach sort_key
fn collect_photos_as_groups(
    state: &ProjectState,
    start: usize,
    end: usize,
) -> Vec<PhotoGroup> {
    let photo_index = build_photo_index(state);

    // Photo-IDs aus dem Bereich sammeln
    let page_photo_ids: Vec<&str> = state.layout[start..end]
        .iter()
        .flat_map(|p| p.photos.iter().map(String::as_str))
        .collect();

    // Nach Originalgruppe aufteilen
    let mut groups_map: HashMap<&str, Vec<PhotoFile>> = HashMap::new();
    for id in &page_photo_ids {
        if let Some((pf, group_name)) = photo_index.get(id) {
            groups_map.entry(group_name)
                .or_default()
                .push((*pf).clone());
        }
    }

    // sort_key aus state.photos übernehmen
    let group_sort_keys: HashMap<&str, &str> = state.photos.iter()
        .map(|g| (g.group.as_str(), g.sort_key.as_str()))
        .collect();

    let mut groups: Vec<PhotoGroup> = groups_map.into_iter()
        .map(|(name, files)| PhotoGroup {
            group: name.to_string(),
            sort_key: group_sort_keys.get(name)
                .unwrap_or(&"")
                .to_string(),
            files,
        })
        .collect();

    groups.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));
    groups
}
```

#### Page-Renumbering

```rust
/// Nummeriert alle LayoutPage.page Felder sequenziell (1-basiert).
/// Nötig nach splice bei Range-Rebuild mit flex, wenn die Seitenanzahl sich ändert.
fn renumber_pages(layout: &mut [LayoutPage]) {
    for (i, page) in layout.iter_mut().enumerate() {
        page.page = i + 1;
    }
}
```

### Modulstruktur-Anpassung

`shared.rs` wird als neues Modul unter `commands/` eingeführt:

```text
src/commands/
  shared.rs       # NEU: rebuild_single_page (von build und rebuild genutzt)
  build.rs        # Nutzt shared::rebuild_single_page
  rebuild.rs      # Nutzt shared::rebuild_single_page
  ...
```

In `src/commands.rs`:

```rust
pub(crate) mod shared;  // Nicht öffentlich, nur für commands-interne Wiederverwendung
```

---

## Implementierungsreihenfolge

Setzt voraus, dass Build-Plan Schritte 1-7 abgeschlossen sind.

| #   | Schritt | Modul | Abhängig von |
| --- | ------- | ----- | ------------ |
| 1 | `rebuild_single_page` aus build.rs nach `shared.rs` extrahieren | `commands/shared.rs` | Build fertig |
| 2 | `rebuild_single` (SinglePage-Scope) | `commands/rebuild.rs` | 1 |
| 3 | `collect_photos_as_groups`, `renumber_pages` | `commands/rebuild.rs` | — |
| 4 | `rebuild_range` (Range-Scope inkl. flex) | `commands/rebuild.rs` | 3 |
| 5 | `rebuild_all` (All-Scope) | `commands/rebuild.rs` | — |

Jeder Schritt = ein Commit. Tests vor jedem Commit.

## Konventionen

- **Conventional Commits**: z.B. `refactor: extract rebuild_single_page to shared module`, `feat: implement rebuild range with flex`
- **Tests**: Unit-Tests + Integrationstests für jeden Schritt
- **`mod solver` unberührt**: Einziger Einstiegspunkt `solver::run_solver`
- **Dateigröße**: Bei >300 Zeilen `rebuild.rs` in Submodule aufteilen

## Tests

| Test | Prüft |
| ---- | ----- |
| SinglePage: nur `layout[n].slots` geändert, andere Seiten unverändert | Keine Seiteneffekte |
| SinglePage: Seite 0 oder > len → Fehler | Validierung |
| Range: `layout[start..end]` komplett neu, umliegende Seiten unverändert | splice-Korrektheit |
| Range flex=0: Seitenzahl bleibt gleich | page_min == page_max == n |
| Range flex>0: Seitenzahl darf variieren, pages danach korrekt renummeriert | renumber_pages |
| Range: Gruppenrekonstruktion erhält ursprüngliche Gruppenzugehörigkeit | collect_photos_as_groups |
| All: komplette Neuverteilung, alle layout-Einträge überschrieben | Vollständiger Reset |
| All: bisher unplaced Fotos werden einbezogen | state.photos als Quelle |
| Rebuild ohne Layout (außer All) → Fehler | Validierung |
| Git pre/post-commits mit korrekten Messages | commit_if_changed |
