# Umsetzungsplan 4 — Solver-Modul

> Konkretisiert Abschnitt 4 des [Hauptdokuments](./README.md).
> Stand: gegen den aktuellen Code geprüft, **noch nichts davon umgesetzt**.

## Worum es geht

Das `solver/`-Modul rechnet korrekt, aber die **Modul-/Namensstruktur** ist
uneinheitlich: Modul-Inception, gemischte Benennung (Algorithmus vs. Aufgabe),
ein Alias, der denselben Typ mehrfach benennt, eine versteckte Datenebene und ein
leicht aufgeblähtes `Request`. Fast alles sind **verhaltensneutrale** Umbenennungen
und Verschiebungen; nur `Request` (H6) und die Config-Punkte (H7/H8) berühren
Signaturen bzw. das YAML-Schema.

## Was offen ist (verifiziert)

- **Inception:** `src/solver/solver.rs` (`#[allow(clippy::module_inception)]`,
  `solver.rs:17`) und `src/solver/ga_solver/solver.rs`.
- **Benennung mischt Achsen:** Aufgabe (`page_layout_solver`, `book_layout_solver`,
  `cover_solver`) vs. Algorithmus (`ga_solver`, `bellman_solver`, `affine_solver`).
  `ga_solver`/`bellman_solver` sind generische, domänenfreie Frameworks.
- **`Params`-Alias:** `BookLayoutSolverConfig as Params` 6×.
- **Versteckte Datenebene:** `mod data_models` (`solver.rs:12`, nicht `pub`), aber
  via `prelude.rs:1` überall sichtbar; Konvertierungen mit Logik in den Modellen
  (`Photo::from_photo_groups`, `SolverPageLayout::to_layout_page`).
- **Große Dateien:** `tree.rs` (654 Z.), `data_models/layout.rs` (694 Z., inkl.
  Bleed-Skalierung `:259–320`); `GAPageEvaluator` im Fassadenfile
  `book_layout_solver.rs:131`.
- **Hardcodiert:** `run_ga` mit `tournament_size = 3`, `seed`/`timeout`-TODOs
  (`page_layout_solver.rs:56,64`).
- **Config:** Feld `ProjectConfig.page_layout_solver: GaConfig`
  (`project_config.rs:10`) wird auch multipage genutzt → Name irreführend;
  uneinheitliche Submodul-Sichtbarkeit (`ga_solver` `pub mod` vs.
  `page_layout_solver` `mod`).

## Commit-Portionen

### Benennung (verhaltensneutral, nur `use`-Pfade)

**H1 · `refactor(solver): resolve module inception`**
`solver/solver.rs` auflösen — Inhalt nach `solver.rs` heben oder Datei in `run.rs`
umbenennen; `ga_solver/solver.rs` (die `GeneticAlgorithm`-Struct) → `engine.rs`.
`#[allow(clippy::module_inception)]` entfällt.
*Verify:* Tests grün; `clippy` ohne das `allow`.

**H2 · `refactor(solver): separate generic algorithms from task solvers`**
Generische Frameworks erkennbar machen: `ga_solver` → `genetic_algorithm`,
`bellman_solver` → `bellman_dp`, gebündelt unter `solver/algorithms/`. Den
algorithmisch benannten `page_layout_solver/affine_solver.rs` nach Aufgabe
benennen (Sibling-Konsistenz zu `fitness`/`tree`/`evolution`). Modul-Docs ergänzen,
welche Module Infrastruktur und welche Domäne sind.
*Verify:* Tests grün.

**H3 · `refactor(solver): drop the Params alias`**
Die 6× `BookLayoutSolverConfig as Params` durch den echten Namen ersetzen (ein
Name pro Typ).
*Verify:* Tests grün.

### Schichten & Dateigröße (verhaltensneutral)

**H4 · `refactor(solver): make the data layer explicit and logic-free`**
`mod data_models` → `pub(crate) mod` (bzw. Modul-Doc, die es als fundamentale
Datenebene ausweist). Konvertierungen an *eine* Grenzschicht `solver/conversion.rs`
ziehen; insbesondere die Sortierlogik aus `Photo::from_photo_groups` und
`SolverPageLayout::to_layout_page` aus den Datenmodellen herausnehmen.
*Verify:* Datenmodelle ohne Geschäftslogik; Tests grün.

**H5 · `refactor(solver): split oversized files`**
Bleed-Skalierung aus `data_models/layout.rs:259–320` in ein eigenes File
(`bleed.rs`/`output_transform.rs`). `GAPageEvaluator` aus
`book_layout_solver.rs:131` in ein Submodul. `tree.rs` bei Bedarf weiter teilen.
Reine Verschiebung.
*Verify:* keine Logikänderung; Tests grün.

### API & Config (sensibel — Aufrufer/Schema betroffen)

**H6 · `refactor(solver): slim the Request type`**
`config: &BookLayoutSolverConfig` ist für `RequestType::SinglePage` ein ungenutztes
Pflichtfeld. Variante: `RequestType` trägt variantenspezifische Daten
(`MultiPage { config }` vs. `SinglePage`), oder die beiden Configs in einen
`SolverConfig` bündeln. Alle Aufrufer mitziehen (`build_layout/single_page.rs`,
`multi_page.rs`, `cover_page.rs`).
*Verify:* Aufrufer kompilieren; Tests grün.

**H7 · `feat(ga): move hardcoded GA params into GaConfig`**
`tournament_size`, `seed`, `timeout` als Felder in `GaConfig` (DTO) — mit
serde-Defaults, damit bestehende YAML weiter lädt. Der Default für `seed` bleibt
`42`, damit die GA-Ausgabe deterministisch identisch bleibt. Entfernt die TODOs in
`run_ga`.
*Verify:* gleiche Layout-Ausgabe bei Default-Config (Golden-/Snapshot-Test); alte
YAML deserialisiert.

**H8 · `refactor(config): rename page_layout_solver field to ga_config`**
Feld in `ProjectConfig` umbenennen, **`#[serde(alias = "page_layout_solver")]`**
für Abwärtskompatibilität bestehender Projekte. Submodul-Sichtbarkeit angleichen
(`ga_solver`/`page_layout_solver` einheitlich `mod` bzw. `pub(crate) mod`).
*Verify:* alte YAML lädt über den Alias; Tests grün.

## Reihenfolge & Risiko

- H1–H5 sind verhaltensneutral (Umbenennen/Verschieben) → zuerst, je einzeln
  reviewbar; Risiko gering, durch die bestehenden Solver-/Proptests abgesichert.
- **H6** ändert eine öffentliche Solver-Signatur → alle Aufrufer im selben Commit.
- **H7/H8** ändern das **Config-/YAML-Schema**: serde-Defaults bzw. `alias`
  zwingend, sonst brechen bestehende Projektdateien. Determinismus (`seed = 42`)
  über einen Vergleichstest absichern, bevor `run_ga` umgestellt wird.
