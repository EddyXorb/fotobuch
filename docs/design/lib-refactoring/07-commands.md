# Umsetzungsplan 7 — Commands

> Konkretisiert Abschnitt 7 des [Hauptdokuments](./README.md).
> Folgt den State-Regeln aus [`../RULES.md`](../RULES.md).
> Stand: gegen den aktuellen Code geprüft.

## Worum es geht

`commands/` hat duplizierte Queries, ein paar Namenskollisionen, uneinheitliche
Config/Result-Muster und zwei übergroße Dateien. Außerdem ist die
**Slot-Move-Familie** (move/swap/combine/split) inkonsistent geschnitten, obwohl
sie ein gemeinsames Fundament teilt.

## Designentscheidung: Slot-Move-Familie (move/swap/combine/split)

Gemeinsames Fundament existiert bereits: Adress-Typen (`Src`, `DstMove`/`DstSwap`,
`PagesExpr`, `SlotExpr`), `PageMoveResult`, `ValidationError` (inkl.
`CombineSinglePage`/`SplitAtFirstSlot`) und `helpers.rs`. **Gewählt: ein Submodul
je Verb** unter `page/`, das Fundament bleibt geteilt in `types.rs`/`helpers.rs`.
Kein Mega-Einstiegs-Enum; jedes Verb hat seinen `execute_<verb>` (Konvention
„Kommando = gleichnamige Funktion").

## Was offen ist (verifiziert)

- **7.1 Duplizierung:**
  - „unplaced finden" 3× (`remove.rs:184 collect_unplaced_ids`,
    `place.rs:56 find_unplaced`, `status.rs:81 count_unplaced`).
  - „leere Seiten entfernen" 2× (`remove.rs:146 remove_empty_pages` ohne Rückgabe
    vs. `page/helpers.rs:74 delete_empty_pages` → `Vec<u32>`, schon von `unplace`/
    `move` genutzt).
  - Page-Listen-Formatter 2× (`place.rs:323 format_page_list` `usize`,`, ` vs.
    `page/helpers.rs:201 format_pages_list` `u32`,`,`).
  - `StateManager::open`/`finish` + „nothing to do"-Frühreturn an ≥8 Stellen.
- **7.2 zu große Dateien:** `move_cmd.rs` (1112 Z., ~605 ohne Tests) mit
  `execute_move_to` / `execute_move_to_manual` / `execute_swap`; `place.rs`
  (625 Z.) mischt Orchestrierung + chronologischen Algorithmus; `remove.rs`
  (583 Z.).
- **7.3 Datei-vs-Ordner:** `add/` ist Ordnermodul, `remove.rs`/`place.rs` (>580 Z.)
  Einzeldateien, `config/`/`project/` Ordner — kein Kriterium.
- **7.4 Config/Result:** `StatusReport` statt `…Result` (`status.rs:57`); kein
  Config-Struct bei `history`/`undo`/`unplace`; `unplace` gibt fremdes
  `PageMoveResult`; `PageMoveCmd` ist Config-Enum.
- **7.5 Namenskollisionen:** `SlotInfo` 2× (`page/types.rs:250` vs.
  `status.rs:31`); `ProjectState_` (`status.rs:20`, Enum `{Empty,Clean,Modified}`);
  `config()` = Modul- und Typname.
- **Überlappung:** `DstMove::Unplace` (`move_cmd.rs:59`) dupliziert
  `execute_unplace` (`unplace.rs:17`).

## Commit-Portionen

### Aufräumen & Namen (verhaltensneutral, zuerst)

**K1 · `refactor(commands): rename colliding types`** (7.5)
`SlotInfo` → `StatusSlotInfo` / `PageSlotInfo`; `ProjectState_` → `ProjectStatus`;
`config()`-Kollision auflösen (z. B. `current_config()`).

**K2 · `refactor(commands): one unplaced-photos query`** (7.1)
Die drei Varianten auf eine Query vereinen (neutral in `models`/Command-Helfer);
`remove`/`place`/`status` delegieren.

**K3 · `refactor(commands): dedupe empty-pages + page-list formatter`** (7.1)
`remove_empty_pages` durch `delete_empty_pages` ersetzen; `format_page_list` →
`format_pages_list` (Trenner vereinheitlichen).

### Struktur: Slot-Move-Familie & große Dateien

**K4 · `refactor(page): split swap out of move_cmd`** (7.2)
Nur `swap` abspalten → `swap.rs` (`execute_swap` + `block_transpose_pages`/
`swap_photos_in_layout` + Contiguity-Helfer). Der **Manual-Drop bleibt in
`move_cmd.rs`** — `DstMove::ManualAt` ist inhaltlich ein Move-*Ziel*, kein eigenes
Verb; `move_cmd.rs` behält Move-to-page/-newpage/-manual/-unplace. `PageMoveCmd`-
Dispatch entfällt bzw. wird reine CLI-Parse-Hülle.
*Namen:* Funktions-Einstiege bleiben `execute_*` (uniform mit `execute_move`),
weil `move` ein Keyword ist (`fn move`/`mod move` unmöglich) — der `execute_`-
Präfix hält die Familie konsistent. Dateien: `swap.rs`/`combine.rs`/`split.rs`
suffixlos, `move_cmd.rs` die eine Keyword-Ausnahme.
*Verify:* reine Verschiebung; Tests grün.

**K5 · `refactor(page): one unplace implementation`**
`DstMove::Unplace` und `execute_unplace` auf eine gemeinsame Routine
zusammenführen.

**K6 · `refactor(place): extract chronological placement algorithm`** (7.2)
`compute_page_ranges`/`find_target_page`/`place_chronologically` in ein
seiteneffektfreies `page_placement.rs` ziehen (isoliert testbar).

**K7 · `refactor(commands): apply a file-vs-folder criterion`** (7.3)
Kriterium festlegen (z. B. „Ordner ab Submodul oder > N Zeilen") und auf
`remove.rs`/`place.rs` anwenden.

### Muster & Wrapper

**K8 · `refactor(commands): unify Config/Result pattern`** (7.4)
`StatusReport` → `StatusResult` (oder Ausnahme dokumentieren); `unplace` mit
eigenem Result; fehlende Config-Structs ergänzen; `PageMoveCmd` als bewusste
Ausnahme dokumentieren.

**K9 · `refactor(commands): run_command wrapper for open/finish`** (7.1)
Open → Ausführung → finish + „nothing to do"-Frühreturn in einen Wrapper. Der
Wrapper **ist** die Orchestrierungs-Spitze (RULES R3) — er hält `mgr` und reicht
schmale Borrows/Views nach unten. Optionaler Hook für Autobuild-after-edit (§13).

## Reihenfolge & Risiko

- K1–K3 verhaltensneutral (Rename/Dedup) → zuerst, breit durch Tests gedeckt.
- K4/K5 strukturell, aber mechanisch (Verschieben/Zusammenführen).
- K6 braucht Sorgfalt (Algorithmus herauslösen) → Test-Abdeckung vorher prüfen.
- K8/K9 Muster-Arbeit; K9 abhängig von den State-Views aus Plan 6 (J11).
