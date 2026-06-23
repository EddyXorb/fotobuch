# Umsetzungsplan 9 — Namens- & Sprachkonsistenz

> Konkretisiert Abschnitt 9 des [Hauptdokuments](./README.md).
> Stand: gegen den aktuellen Code geprüft.

## Worum es geht

Querschnitt-Aufräumen: eine Sprache pro Schicht, eine Verb-Konvention für
Command-Einstiege, ein Verb pro Cache-Bedeutung. Alles verhaltensneutral.

## Bereits erledigt (durch #45/#48/#50)

- `Report` → `Result` (`StatusResult`), `Params`-Alias entfernt,
  `dto_models` → `models`, `update_preview_cache`/`_pdf` existieren nicht mehr.

## Was offen ist (verifiziert)

- **Deutsch im Code** (Bezeichner englisch, einzelne Doc-/Zeilenkommentare
  deutsch), nur noch lokal: `commands/remove/matchers.rs:15,16,51,52,104,105`,
  `commands/build/helpers.rs:139,153`, `output/render.rs:49`. Typo `nothting` in
  `solver/algorithms/genetic_algorithm/evolution.rs:16`.
- **Verb-Konvention der Command-Einstiege** dreifach: bare (`add`/`place`/
  `remove`/`status`/`build`/`history`/`undo`/`redo`), `execute_*` (alle
  `page/`-Commands **und** `execute_unplace`), `project_*` (`project_list`/
  `project_new`/`project_switch`).
- **Cache-Verben** gemischt: `ensure_previews` vs. `build_final_cache` für
  dieselbe Rolle „Cache erzeugen" (die `refresh_*`-Wrapper sind konsistent).

## Konvention für Command-Einstiege (Entscheidung)

`move` ist ein Keyword (`fn move` unmöglich) → die `page/`-Familie bleibt
einheitlich auf `execute_*`. Daraus die Regel:

- **bare verb** für Top-Level-Commands (`add`, `place`, `remove`, …) **inkl.
  `unplace`**.
- **`execute_*`** nur für die `page/`-Familie (keyword-getrieben, intern uniform).
- **bare verb innerhalb des Moduls** für `project/`: das Modul namensraumt bereits
  → `project::new`/`list`/`switch` statt `project_*` (kein „project_project_new").

## Commit-Portionen

**M1 · `refactor(commands): rename unplace/project entries to the verb convention`**
`execute_unplace` → `unplace`; `project_new`/`project_list`/`project_switch` →
`new`/`list`/`switch` (innerhalb `commands/project/`). Aufrufer in `cli`/`gui`
nachziehen.
*Verify:* `cli`/`gui` kompilieren; Tests grün.

**M2 · `refactor: translate remaining German comments to English`**
Die o. g. Stellen in `remove/matchers.rs`, `build/helpers.rs`, `output/render.rs`
übersetzen. Prosa-Doku unter `docs/` bleibt deutsch.

**M3 · `fix: typo nothting → nothing`**
`evolution.rs:16`.

**M4 · `refactor(cache): one verb for cache generation`**
`ensure_previews` und `build_final_cache` auf *ein* Verb bringen (z. B.
`build_preview_cache`/`build_final_cache` oder `ensure_*`/`ensure_*`). Die
`refresh_*`-Wrapper und Aufrufer nachziehen.
*Verify:* Tests grün.

## Reihenfolge & Risiko

Durchweg gering (Umbenennungen/Übersetzungen). M1/M4 berühren Aufrufer in
`cli`/`gui` → im selben Commit mitziehen; M2/M3 rein lokal.
