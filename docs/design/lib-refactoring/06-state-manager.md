# Umsetzungsplan 6 — `state_manager`

> Konkretisiert Abschnitt 6 des [Hauptdokuments](./README.md).
> Stand: gegen den aktuellen Code geprüft.

## Worum es geht

`StateManager` ist die zentrale Schnittstelle zwischen Commands und dem
persistierten Zustand. Die meisten Befunde sind kleine, verhaltensneutrale
Aufräumarbeiten (Toter Code, Sichtbarkeit, Doc-Drift). Zwei Punkte sind
strukturell größer (Kapselung von `state`, zwei Ladewege) und brauchen eine
Designentscheidung.

## Was sich seit der Skizze geändert hat

- **`renumber_pages` ist jetzt ein No-Op** (`state_manager.rs:27`): seit das
  Feld `LayoutPage.page` in #48 entfernt wurde, ist die Array-Position die
  kanonische Identität. Die Funktion tut nichts mehr — aus „kapseln" wird
  „löschen".
- `dto_models` heißt jetzt `models` (Import in `state_manager.rs:22`).

## Was offen ist (verifiziert)

- **Toter No-Op `renumber_pages`** mit drei Aufrufstellen, die nur dafür je ein
  `has_cover` berechnen (`state_manager.rs:216`, `commands/remove.rs:252`,
  `commands/build/plan.rs:72`) + zwei Importe.
- **`finish_always` gibt `Result<Option<ProjectState>>`** (`:201`), committet
  aber laut Logik (`finish_internal`, `always_commit = true`) **immer** → das
  `Option` ist hier strukturell überflüssig.
- **Doc-Drift:** `ensure_build_baseline` (`:287`) nennt im Doc den Zustand
  `"NoBuildCommit"`, die reale Variante heißt `LazyLoad::Failed`.
- **Übergroße Sichtbarkeit in `state_diff.rs`:** `count_config_changes`,
  `count_value_diffs`, `diff_photos`, `diff_pages` (`:82,88,108,138`) sind `pub`,
  werden aber nur von `StateDiff::compute` im selben Modul genutzt.
- **`page_change_detection.rs`** (1021 Z.) ist zu ~75 % Tests (`mod tests` ab
  `:252`); nur `compute_outdated_pages` ist öffentlich. Inhaltlich ok,
  Test-Fixtures extrahierbar.

### Strukturell größer (Designentscheidung nötig)

- **`mgr.state: pub`** (`state_manager.rs:54`) — 186 schreibende/lesende
  `mgr.state.*`-Zugriffe crate-weit. Das `pub` ist **bewusst** gesetzt (Doc
  `:45`): Commands brauchen *disjunkte* Borrows auf `state.photos` und
  `state.layout` zugleich. Eine naive Kapselung (privat + Getter) bricht genau
  das. Invarianten (Validität, Numerierung) sind dadurch ungeschützt.
- **Zwei Ladewege ohne Typschutz:** `StateManager::open` (Lifecycle,
  Auto-Commit, Drop-Warnung) vs. freie `load_project_state` (`:388`, ohne
  Garantien), genutzt von `undo`/`switch`. Nichts verhindert die falsche Wahl.
  `open()` hat zudem einen kaum sichtbaren Seiteneffekt
  (`auto_commit_manual_edits`, `:119`).

## Commit-Portionen

### Verhaltensneutrale Aufräumarbeiten

**J1 · `refactor(state): remove dead renumber_pages no-op`**
Funktion (`:27`) und alle drei Aufrufstellen löschen, samt der nur dafür
berechneten `has_cover`-Locals und der zwei Importe (`remove.rs:10`,
`plan.rs:8`). Reine Streichung — die Funktion tut nichts.
*Verify:* kompiliert ohne die Importe; Tests grün.

**J2 · `refactor(state): drop the redundant Option from finish_always`**
`finish_always` → `Result<ProjectState>` (committet immer). `finish` behält
`Option` (echter No-Op-Fall). Internen Helfer ggf. aufteilen oder Rückgabe am
`always_commit`-Zweig auspacken. Aufrufer von `finish_always` mitziehen.
*Verify:* Aufrufer kompilieren; `finish`-No-Op-Test bleibt grün.

**J3 · `refactor(state): tighten state_diff helper visibility`**
Die vier nur intern genutzten Helfer von `pub` auf modulprivat setzen.
*Verify:* kompiliert; Tests grün.

**J4 · `docs(state): fix ensure_build_baseline doc drift`**
`"NoBuildCommit"` → `Failed` im Doc-Kommentar.

### Optional / niedrige Priorität

**J5 · `test(state): extract page_change_detection fixtures`**
Die ~770 Test-Zeilen in ein `tests/`-Fixture-/Helper-Modul ziehen, damit die
Logik-Datei lesbar bleibt. Reine Testumstrukturierung.
*Verify:* Tests unverändert grün.

## Zu evaluieren (größere Eingriffe, nicht ohne Designentscheidung)

- **Schreibzugriff auf `state` kanalisieren.** Ziel: Invarianten an einer Stelle
  durchsetzen, ohne disjunkte Borrows zu verlieren. Optionen:
  - Ein borrow-freundlicher Accessor `state_mut() -> (&mut Vec<PhotoGroup>, &mut
    Vec<LayoutPage>, …)`, der die heute nötigen Doppel-Borrows abdeckt, und
    `state` danach auf `pub(crate)`/privat ziehen.
  - Schreibende Sammeloperationen als Methoden auf `StateManager` mit
    anschließendem `check_validity` (z. B. „Seiten entfernen", „Foto platzieren").
  - Mindestens: `state` auf `pub(crate)` einschränken (GUI/CLI greifen dann nur
    über Commands zu). Nutzen/Aufwand bei 186 Stellen abwägen.
- **Zwei Ladewege vereinheitlichen.** `load_project_state` ist ein bewusster
  Read-only-Pfad ohne Lifecycle (undo/redo/switch). Entweder klar als solcher
  benennen/dokumentieren (`read_state_readonly`) oder hinter einen schlanken
  Read-only-Typ legen, sodass die Wahl typsicher wird. `open()`s Auto-Commit-
  Seiteneffekt explizit dokumentieren oder benennen.

## Reihenfolge & Risiko

- J1–J4 sind verhaltensneutral und einzeln reviewbar → zuerst; durch die
  bestehenden StateManager-/Diff-Tests abgedeckt.
- J5 ist reine Testumstrukturierung.
- Die „Zu evaluieren"-Punkte berühren viele Aufrufer (186 `state`-Zugriffe) →
  erst nach Designentscheidung, ggf. eigener Plan.
