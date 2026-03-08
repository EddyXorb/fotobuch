# Implementation Plan: `fotobuch rebuild`

Stand: 2026-03-08

## Überblick

Erzwingt Neuberechnung — mächtiger als `build`. Unterstützt Einzelseite, Seitenbereich und kompletten Neustart.

## Abhängigkeiten

- `commands/build.rs` (Preview-Cache und Typst-Logik wiederverwenden)
- `solver` (vorhanden)
- `git::commit` (vorhanden)

## Ablauf je Scope

### `RebuildScope::SinglePage(n)`

1. Git pre-commit: `pre-rebuild: page {n}`
2. Preview-Cache prüfen (wie build)
3. **`run_solver` SinglePage** erzwungen — auch wenn `build` die Seite als clean einstufen würde:

   ```rust
   let group = PhotoGroup { files: photos_of_page(state, n), .. };
   let pages = run_solver(&Request {
       request_type: RequestType::SinglePage,
       groups: &[group],
       config: &state.config.book_layout_solver,
       ga_config: &state.config.page_layout_solver,
       book_config: &state.config.book,
   })?;
   state.layout[n - 1].slots = pages[0].slots.clone();
   ```

4. YAML schreiben, Typst kompilieren
5. Git post-commit: `post-rebuild: page {n} (cost: {cost})`

### `RebuildScope::Range { start, end, flex }`

1. Git pre-commit: `pre-rebuild: pages {start}-{end}`
2. Preview-Cache prüfen
3. **`run_solver` MultiPage** auf Teilmenge mit angepassten Seitengrenzen:

   ```rust
   let n = end - start + 1;
   let config = BookLayoutSolverConfig {
       page_min: n.saturating_sub(flex).max(1),
       page_max: n + flex,
       page_target: n,
       ..state.config.book_layout_solver.clone()
   };
   let groups = photos_from_page_range(state, start, end); // als PhotoGroups
   let pages = run_solver(&Request {
       request_type: RequestType::MultiPage,
       groups: &groups,
       config: &config,
       ga_config: &state.config.page_layout_solver,
       book_config: &state.config.book,
   })?;
   state.layout.splice((start - 1)..end, pages);
   ```

4. YAML schreiben, Typst kompilieren
5. Git post-commit: `post-rebuild: pages {start}-{end} (cost: {total_cost})`

### `RebuildScope::All`

1. Git pre-commit: `pre-rebuild: all`
2. Preview-Cache prüfen
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

4. YAML schreiben, Typst kompilieren
5. Git post-commit: `post-rebuild: {p} pages (cost: {total_cost})`

## Wiederverwendung aus `build`

Die Preview-Cache-Logik und Typst-Kompilierung sollen aus `build.rs` in gemeinsame Hilfsfunktionen extrahiert werden:

```rust
// commands/build.rs (oder cache.rs / output.rs)
pub(crate) fn ensure_cache_and_compile(
    state: &ProjectState,
    project_root: &Path,
    release: bool,
) -> Result<PathBuf>
```

`rebuild` ruft diese intern auf.

## `--flex` Constraint

`flex` wird als `page_min` / `page_max` im `BookLayoutSolverConfig` an `run_solver` übergeben — siehe Range-Ablauf oben. Bei `SinglePage` und `All` wird flex ignoriert.

## Konventionen

- **Conventional Commits**: Jeder Teilschritt bekommt einen eigenen Commit (z.B. `feat: implement rebuild single page via run_solver`, `feat: implement rebuild range with flex constraint`).
- **Tests vor jedem Commit**: `cargo test` muss fehlerfrei durchlaufen — kein Commit mit roten Tests.
- **Tests schreiben**: Für jeden neuen Teilschritt sind Unit-Tests (`#[cfg(test)]` inline) und Integrationstests (`tests/`) Pflicht.
- **`mod solver` unberührt**: Keine Änderungen in `src/solver/`. Der einzige Einstiegspunkt ist `solver::solver::run_solver` — `rebuild` ruft ausschließlich diese Funktion auf.
- **Dateigröße**: Bei >300 Zeilen `rebuild.rs` in `rebuild/` aufteilen (z.B. `rebuild/single.rs`, `rebuild/range.rs`).

## Tests

- Einzelseite: nur `layout[n].slots` geändert, andere Seiten unverändert
- Range: `layout[start..end].photos` neu gesetzt
- Range mit flex: Seitenzahl darf variieren
- All: komplette Neuverteilung, alle vorherigen layout-Einträge überschrieben
- Git-Commits vor und nach dem Solver
