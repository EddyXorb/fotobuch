# Umsetzungsplan 6 — `state_manager`

> Konkretisiert Abschnitt 6 des [Hauptdokuments](./README.md).
> Setzt die State-Zugriffsregeln aus [`../RULES.md`](../RULES.md) um.
> Stand: gegen den aktuellen Code geprüft.

## Worum es geht

Zwei Stränge: (a) **kleine Aufräumarbeiten** (toter Code, Sichtbarkeit,
Doc-Drift) und (b) das **Kernthema Kapselung** — `mgr.state: pub` (186 Zugriffe)
erlaubt beliebiges Mutieren, und der Footprint einer Änderung ist „von weitem"
unsichtbar. (b) setzt die in `RULES.md` festgehaltenen Regeln R1–R6 um.

## Designentscheidungen (Kurzform, Details in `RULES.md`)

- **R1 Trichotomie:** `state() -> &ProjectState` (Domänen-Lesen) · Manager-Lesen
  (git/Pfade) nur an der Spitze · schmale `*_mut()` zum Schreiben. Kein
  `&mut ProjectState`, kein `state_mut()`.
- **R3 Wirbelsäule:** `mgr` nur auf der Orchestrierungs-Spitze; nie eine Schicht
  tiefer; schmale Borrows nach unten.
- **R4 Narrow early:** Orchestrator-Muster Lesen→entscheiden→schreiben; Lese-
  Abgeleitetes vorher als *owned* ziehen.
- **R5 Read-only** als eigener Typ via `open_readonly()`.

## Was offen ist (verifiziert)

- **Toter No-Op `renumber_pages`** (`:27`) + 3 Aufrufstellen (`:216`,
  `remove.rs:252`, `build/plan.rs:72`) + 2 Importe.
- **`finish_always`** gibt `Result<Option<ProjectState>>` (`:201`), committet aber
  immer.
- **Doc-Drift** `ensure_build_baseline` (`:287`): nennt `"NoBuildCommit"`, real
  `LazyLoad::Failed`.
- **`state_diff.rs`-Helfer `pub`** (`:82,88,108,138`), nur modulintern genutzt.
- **`page_change_detection.rs`** (1021 Z.) zu ~75 % Tests (`mod tests` ab `:252`).
- **`mgr.state: pub`** (`:54`) + freie `load_project_state` (`:388`, von
  undo/redo/switch genutzt) ohne Typschutz.
- **Wirbelsäulen-Verstoß build-Pfad:** `build_layout` reicht `mgr` an
  `build_full_book`/`build_page`/`build_outdated_pages`/`build_page_range` weiter;
  Solver-Engines `solve_multipage`/`solve_single_page` nehmen `&mut ProjectState`.

## Commit-Portionen

### Aufräumen (verhaltensneutral, vorab)

**J1 · `refactor(state): remove dead renumber_pages no-op`**
Funktion + 3 Aufrufstellen + die nur dafür berechneten `has_cover`-Locals + 2
Importe löschen.

**J2 · `refactor(state): drop the redundant Option from finish_always`**
`finish_always` → `Result<ProjectState>`; `finish` behält `Option` (echter
No-Op-Fall). Aufrufer mitziehen.

**J3 · `refactor(state): tighten state_diff helper visibility`**
Die vier nur intern genutzten Helfer auf modulprivat setzen.

**J4 · `docs(state): fix ensure_build_baseline doc drift`**
`"NoBuildCommit"` → `Failed`.

### Kapselung (setzt R1–R5 um)

> Reihenfolge so gewählt, dass jeder Schritt für sich kompiliert und testbar ist.

**J5 · `refactor(state): add read surface + open_readonly handle`**
Lese-Oberfläche (`state()`, Pfade, `outdated_pages_indices()`) in einen geteilten
Lese-Kern ziehen, den `StateManager` enthält. `open_readonly() -> ReadHandle`
ohne `finish`/`*_mut`/Auto-Commit/Drop-Warnung. `load_project_state` entfällt;
`undo`/`redo`/`project switch` migrieren auf `open_readonly`.
*Verify:* read-only-Konsumenten kompilieren; kein Verhaltenswechsel; Tests grün.

**J6 · `refactor(state): channel writes through narrow field accessors`**
`layout_mut()`/`photos_mut()`/`config_mut()` einführen; `state` von `pub` auf
`pub(crate)`/privat (Lesen über `state()`). Vorhandene `&mut mgr.state.<feld>`-
Stellen (~17) darauf umstellen. **Kein** `state_mut()`.
*Verify:* nichts mutiert mehr `state` direkt von außen; Tests grün.

**J7 · `refactor(solver): narrow solve engines off &mut ProjectState`** (R1)
`solve_multipage`/`solve_single_page`: `&mut ProjectState` → `&mut Vec<LayoutPage>`
(write) + `&BookConfig`/`&BookLayoutSolverConfig`/`&[PhotoGroup]` (read). `SolverPlan`
entsprechend mit schmalen Borrows speisen.
*Verify:* gleiche Layout-Ausgabe (Solver-/Snapshot-Tests); Tests grün.

**J8 · `refactor(build): keep mgr on the spine, pass narrow borrows down`** (R3/R4)
- `outdated_pages_indices()` aus `build_outdated_pages` in `build_layout`/`run`
  heben (Manager-Lesen an der Spitze), owned `Vec<usize>` nach unten (B1).
- `build_full_book`/`build_page`/`build_page_range`/`build_outdated_pages`
  verlieren `&mut StateManager`, nehmen schmale Borrows; `build_layout` macht die
  `state()`-Reads + `layout_mut()`-Griffe (B3).
*Verify:* build/rebuild-Tests grün; `mgr` nirgends unterhalb der Spitze.

**J9 · `refactor(commands): read→decide→write orchestrators for edit commands`** (R4)
Edit-Helfer (`place_chronologically`, `place_into_*`, `remove_from_*`, …) von
`&mut ProjectState` auf das geschriebene Feld + owned Lese-Args verengen; die
Command-Orchestratoren in Lese-/Schreibphase gliedern (Footprint oben sichtbar).
*Verify:* unverändertes Verhalten; Tests grün.

### Optional

**J10 · `test(state): extract page_change_detection fixtures`**
Die ~770 Testzeilen in ein Fixture-Modul ziehen.

## Reihenfolge & Risiko

- **J1–J4** verhaltensneutral, einzeln reviewbar → zuerst.
- **J5/J6** sind das Fundament der Kapselung (Lese-Kern, Accessoren); danach
  greifen R1–R5 mechanisch.
- **J7** ist der einzige inhaltlich sensible Schritt (Solver-Signaturen) → durch
  Snapshot-Tests absichern.
- **J8/J9** sind Anwendungen der Wirbelsäulen-/Narrow-early-Regel, durch die
  bestehenden build-/command-Tests gedeckt.
- **J10** rein kosmetisch.
