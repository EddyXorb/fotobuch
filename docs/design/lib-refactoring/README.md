# Lib-Refactoring — Strukturvorschläge für `src/`

> Status: Abschnitte 1–10 sind umgesetzt (Details in den separaten Plänen
> `0x-*.md`, siehe Abschnitt 11). Offen sind nur noch Nachzieh-Punkte: J11 aus
> Abschnitt 6, die Benennung aus Abschnitt 3 und Abschnitt 12.2/12.3.
> Reihenfolge je Abschnitt = Priorität. Kein fertiger Code, nur Ideen +
> Signatur-Skizzen. Befunde mit `Datei:Zeile` belegt (offene Abschnitte gegen den
> aktuellen Stand geprüft).

Leitprinzipien für alle Vorschläge:

- **Eine Sache pro Abstraktionsebene.** Eine Methode orchestriert *oder* sie
  rechnet — nicht beides.
- **Konsistente Sprache.** Bezeichner + Doc-Comments durchgehend Englisch,
  Domänenbegriffe exakt wie in `docs/book/src/glossary.md`.
- **Konsistente Namensschemata.** Gleiche Rolle ⇒ gleiches Suffix/Präfix.
- **Eine Wahrheit pro Konzept.** Gleiche Logik nicht mehrfach implementieren.

Verbindliche, ausdiskutierte Regeln (State-Zugriff u. a.) stehen in
[`../RULES.md`](../RULES.md).

Inhalt:
1. Kernproblem: Vielfalt der build-Methoden
2. Cover-Logik konsolidieren
3. Datei- & Modulstruktur (build-Pfad)
4. Solver-Modul
5. Domänenmodell (`dto_models`)
6. `state_manager`
7. Commands: Duplizierung, Struktur, Config/Result-Muster
8. `input`-Modul
9. Querschnitt: Namens- & Sprachkonsistenz
10. Fehlermeldungen & Methoden-Komplexität
11. Umsetzungspläne (separate Dokumente) → [`01-build.md`](./01-build.md)
12. Edit-Commands: Layout-Konsistenz & optionaler Rebuild

---

## 1. Kernproblem: Vielfalt der build-Methoden

**Status: umgesetzt.** Die früheren zwei Einstiegskommandos und sechs
Orchestratoren (`first_build`/`incremental_build`/`release_build`/`rebuild_*`,
`multipage_build`) sind ersetzt durch **einen** `build()`-Einstieg plus die
`BuildPlan`-Pipeline (`Auto`/`All`/`Page`/`Range`/`Release`, dispatcht in
`plan.rs`). `MultiPageParams`, `BuildOptions`, `RebuildScope` und die separate
`rebuild()`-Funktion sind weg; `BuildConfig` trägt den `BuildPlan`; `BuildResult`
ist entschlackt (kein `pages_swapped`/`total_cost` mehr).

Vollständige Analyse, Zielbild-Skizze und der Conventional-Commit-Plan (inkl. der
zuletzt umgesetzten Phase E): [`01-build.md`](./01-build.md).

---

## 2. Cover-Logik konsolidieren

**Status: umgesetzt.** Die früher über `multipage_build` und
`rebuild_single_page` verstreute Cover-Logik (Fallunterscheidung *free vs.
structured*, `split_cover_photos`, `build_cover_page`, `update_cover_page`) liegt
jetzt gebündelt in `commands/build/build_layout/cover_page.rs`.

**Ebenfalls erledigt** (#44) ist die Cover-**Geometrie**: `spread_width_mm` und
`spine_width_mm` liegen jetzt im Wertobjekt `CoverGeometry` (`models/cover.rs`),
das `CoverConfig` an einen konkreten `inner_page_count` bindet und `CanvasConfig`
implementiert; `CoverConfig` ist wieder ein reines Daten-DTO ohne
`CanvasConfig`-Impl. Plan: [`02-cover.md`](./02-cover.md) (verwandt mit
Abschnitt 5.2).

---

## 3. Datei- & Modulstruktur des build-Pfads

**Status: ursprüngliche Befunde erledigt.** `build/core/` und `build/core.rs`
sind weg (Pfad verflacht), `rebuild.rs` ist als `BuildPlan`-Variante in den einen
`build()`-Einstieg aufgegangen, `build_photo_index` lebt jetzt in
`dto_models/photos.rs` (von `place`/`status` dort importiert), die driftende
Doc-Numerierung ist mit `rebuild.rs` verschwunden.

**Offen bleibt** Benennung & Kohäsion im jetzt gut geschnittenen Pfad:
`helpers.rs` ist ein Sammelname mit vier Themen, Modul `build_layout` kollidiert
mit der Methode `BuildPlan::build_layout()`, und die Layout-Schicht mischt die
Verben `build_`/`solve_`/`update_`. Plan: [`03-build-structure.md`](./03-build-structure.md).

---

## 4. Solver-Modul

**Status: umgesetzt** (#45). Vollständiger Plan: [`04-solver.md`](./04-solver.md).
Befunde in Kürze:

- **Modul-Inception:** `solver/solver.rs` (`#[allow(module_inception)]`,
  `solver.rs:17`) und `ga_solver/solver.rs`.
- **Benennung mischt Achsen:** Aufgabe (`page_layout_solver`/`book_layout_solver`/
  `cover_solver`) vs. Algorithmus (`ga_solver`/`bellman_solver`/`affine_solver`);
  die generischen Frameworks gehören erkennbar getrennt.
- **`Params`-Alias** (`BookLayoutSolverConfig as Params`, 6×) → ein Name pro Typ.
- **Versteckte Datenebene** `mod data_models` (nicht `pub`, aber via `prelude`
  überall sichtbar) + Konvertierungslogik in den Modellen.
- **Große Dateien** (`tree.rs` 654 Z., `data_models/layout.rs` 694 Z.),
  `GAPageEvaluator` im Fassadenfile, hardcodierte GA-Parameter in `run_ga`.
- **`Request` leicht aufgebläht**; Config-**Typ** `GaConfig` ist nach dem
  Algorithmus statt der Aufgabe benannt (Sibling: `BookLayoutSolverConfig`) →
  `PageLayoutSolverConfig`. Das Feld `page_layout_solver` bleibt (domänenrichtig).

---

## 5. Domänenmodell (`dto_models` → `models`)

**Status: umgesetzt** (#48; `dto_models` heißt jetzt `models`). Vollständiger
Plan: [`05-dto-models.md`](./05-dto-models.md). Befunde in Kürze:

- **Name `dto_models`** suggeriert Logiklosigkeit, ist aber Domänenmodell.
- **Logik auf DTOs:** `ProjectState` als Gott-Objekt-Keim (Persistence +
  Queries + Validierung), `BookLayoutSolverConfig::validate(total_photos)` auf
  dem Config-DTO, `CanvasConfig`-Trait in einer spezifischen Config-Datei.
- **Schwache Typen/Redundanz:** `aspect_ratio()` 3×, `PhotoGroup.sort_key: String`
  (eigentlich Timestamp), `CoverMode`-Default-Falle (`Free` vs. `Split`),
  Deprecated-Felder im aktiven Config, `BookConfig` mit Pflicht-Kindern,
  denormalisiertes `LayoutPage.page`.

---

## 6. `state_manager`

**Status: umgesetzt** (#49); Nachzieh-Schritt J11 (View-Konsolidierung) offen.
Vollständiger Plan: [`06-state-manager.md`](./06-state-manager.md). Befunde in
Kürze:

- **Toter No-Op `renumber_pages`** (`state_manager.rs:27`, seit Wegfall von
  `LayoutPage.page` funktionslos) plus drei Aufrufstellen → ersatzlos löschen.
- **`finish_always`** gibt `Result<Option<ProjectState>>`, committet aber immer →
  `Option` überflüssig.
- **Doc-Drift** (`ensure_build_baseline` nennt `"NoBuildCommit"` statt `Failed`),
  übergroße `pub`-Sichtbarkeit der `state_diff`-Helfer, Test-lastiges
  `page_change_detection.rs`.
- **Kernthema Kapselung:** `mgr.state: pub` (186 Zugriffe) öffnet beliebiges
  Mutieren; Footprint von Änderungen ist „von weitem" unsichtbar.

**Designentscheidungen** (ausdiskutiert, festgehalten in [`../RULES.md`](../RULES.md)):

- **Trichotomie:** Domänen-Lesen über *ein* breites `state() -> &ProjectState`;
  Manager-Lesen (git-Baseline, Pfade) über `mgr`-Methoden nur an der Spitze;
  Domänen-Schreiben über die View-Konstruktoren `mgr.write_*()` (R7), `*_mut()`
  lebt auf der View. `&mut ProjectState` und ein `state_mut()` entfallen.
- **Wirbelsäulen-Regel:** `mgr` lebt nur auf der Orchestrierungs-Spitze (Pipeline
  + direkte Schritt-Helfer) und wird **nie eine Schicht tiefer** gereicht; schmale
  Borrows gehen nach unten (*narrow early*, Muster Lesen→entscheiden→schreiben).
- **State-Views gegen Signatur-Explosion (R7):** statt vieler Einzel-Parameter ein
  Wrapper über `&mut ProjectState` mit genau *einem* schreibbaren Feld —
  `WriteLayoutState`/`WritePhotosState`/`WriteConfigState` (Typname = Footprint).
  Lesen ohne Schreiben bleibt `&ProjectState` (kein `ReadState`). Keine
  layout+photos-Kombi (kein Bedarf; `remove` mutiert sequenziell).
- **Read-only** als eigener Typ: `open_readonly()` liefert einen schmalen
  Lese-Handle (kein `finish`/`*_mut`) und ersetzt das freie `load_project_state`.
- **Validität** bleibt zentral in `finish()` (Drop warnt); kein Per-Edit-Check.

Kontrolliert: Edit-Commands erfüllen die Wirbelsäulen-Regel bereits; nur der
build-Pfad reicht `mgr` eine Schicht zu tief (`build_layout` → `build_full_book`)
und die Solver-Engines nehmen noch `&mut ProjectState` — beides löst der Plan auf.

---

## 7. Commands: Duplizierung, Struktur, Config/Result-Muster

**Status: umgesetzt** (#50). Vollständiger Plan: [`07-commands.md`](./07-commands.md).
Befunde in Kürze:

- **7.1 Duplizierung:** „unplaced finden" 3×, „leere Seiten entfernen" 2×,
  Page-Listen-Formatter 2×, open/finish-Boilerplate ≥8× → `run_command`-Wrapper.
- **7.2 große Dateien:** `move_cmd.rs` (1112 Z.), `place.rs` (625 Z.,
  Orchestrierung + Algorithmus vermischt), `remove.rs` (583 Z.).
- **7.3 Datei-vs-Ordner** ohne Kriterium; **7.4 Config/Result-Muster**
  uneinheitlich (`StatusReport`, fehlende Config-Structs); **7.5 Kollisionen**
  (`SlotInfo` 2×, `ProjectState_`, `config()`).

**Designentscheidung Slot-Move-Familie:** move/swap/combine/split teilen das
Fundament (Adress-Typen, `PageMoveResult`, `ValidationError`, Helfer); gewählt ist
**ein Submodul je Verb** unter `page/` mit geteiltem Fundament — kein
Mega-Einstiegs-Enum. Dazu `DstMove::Unplace`/`execute_unplace` auf eine
Implementierung zusammenführen. Details im Plan.

---

## 8. `input`-Modul

**Status: umgesetzt** (`763866a`). Vollständiger Plan: [`08-input.md`](./08-input.md).
Befunde in Kürze:

- **Zwei `metadata.rs`, nur über den Pfad unterscheidbar:** `input/metadata.rs`
  (`compute_partial_hash`, Hashing) vs. `input/scan/metadata.rs`
  (`enrich_photo_metadata`, EXIF) → Hashing-Datei umbenennen.
- **`enrich_photo_metadata` als Scan-API exportiert**, aber auch von
  `cache/preview_cache.rs` genutzt → als Enricher auf Input-Ebene rehomen.
- **Kommentar-Leiche** `// pub mod grouper;` (`input.rs:6`).

---

## 9. Querschnitt: Namens- & Sprachkonsistenz

**Status: umgesetzt** (#52; weitere Punkte schon durch #45/#48/#50). Vollständiger
Plan: [`09-naming.md`](./09-naming.md). Befunde in Kürze:

- **Deutsch im Code** nur noch lokal (`remove/matchers.rs`, `build/helpers.rs`,
  `output/render.rs`) + Typo `nothting` (`evolution.rs:16`).
- **Verb-Konvention** dreifach (bare / `execute_*` / `project_*`) → Regel: bare
  für Top-Level inkl. `unplace`, `execute_*` nur für die `page/`-Familie
  (keyword-getrieben), `project::new`/`list`/`switch` statt `project_*`.
- **Cache-Verben** `ensure_previews` vs. `build_final_cache` → ein Verb pro Rolle.

---

## 10. Fehlermeldungen & Methoden-Komplexität

Vollständiger Plan: [`10-errors-and-methods.md`](./10-errors-and-methods.md).
Bündelt zwei verwandte Hygiene-Themen.

**Teil A — CLI-Begriffe aus der Lib verbannen (umgesetzt mit #53).** Die im Plan
gelisteten Fundstellen im build-Pfad, in `page/types.rs`, `project/switch.rs` und
`project/new.rs` sind auf getippte Fehler (`BuildError`, `ProjectError`,
`ValidationError`) bzw. auf CLI-Ausgabe umgestellt; die Hinweise liegen in
`cli/cli/hints.rs` und lesen ihre Flag-Namen aus `Cli::command()`.

Im selben PR nachgezogen (Nachtrag N6 im Plan): die im ursprünglichen Befund
fehlenden Fundstellen unterhalb der Command-Ebene — `ProjectError::CoverWithoutSpine`
(`project/new.rs`), `CoverSolverError::MissingPhoto` (`solver/cover_solver.rs`)
und `StateError::NotOnProjectBranch` (`state_manager.rs`, beide Stellen über
einen Helfer zusammengeführt).

- **Zwei Probleme:** für die GUI (gleiche Lib) sind CLI-Hinweise sinnlos; Flag-
  Namen driften unbemerkt von den clap-Definitionen weg.
- **Designentscheidung:** Bedingung und Abhilfe trennen. Die Lib gibt *getippte*
  Fehler zurück (`thiserror`, `#[error]`-Text ohne Kommandos); die CLI übersetzt
  Variante → Hinweis und liest die Flag-Namen zur Laufzeit aus `Cli::command()`.
  Gegen Drift: exhaustives `match` ohne `_`-Arm (fehlender Hinweis → Compilefehler)
  und ein Test, der jeden clap-Lookup auflöst. Mechanismus-Details (clap-`long()`-
  Helfer, Test) stehen im Plan.

**Teil B — Innerhalb von Methoden (umgesetzt mit #50).** Die früheren build-Punkte
(`multipage_build`-Zweige, `RenderContext`, `incremental_build`,
`apply_page_filter`) sind mit Abschnitt 1 erledigt; der letzte Restpunkt
`execute_move_to` (6 Frühreturns quer durch die Abstraktionsebenen) ist mit der
Slot-Move-Familie aufgelöst — `move_cmd.rs` (Dispatch) / `standard.rs`
(Orchestrierung, R4) / `manual.rs` (Manual-Pfad) sind getrennt.

---

## 11. Umsetzungspläne (separate Dokumente)

Die konkreten, in Conventional Commits gegliederten Umsetzungspläne liegen je
Themenblock in eigenen Dateien neben diesem Dokument:

- [`01-build.md`](./01-build.md) — Vereinheitlichung der build-/rebuild-Methoden
  (Abschnitte 1–3).
- [`02-cover.md`](./02-cover.md) — Cover-Geometrie aus dem DTO lösen
  (Abschnitt 2 / 5.2; Konsolidierung bereits erledigt).
- [`03-build-structure.md`](./03-build-structure.md) — Benennung & Kohäsion im
  build-Pfad (Abschnitt 3; Strukturbefunde bereits erledigt).
- [`04-solver.md`](./04-solver.md) — Modul-/Namensstruktur des Solvers
  (Abschnitt 4; umgesetzt).
- [`05-dto-models.md`](./05-dto-models.md) — Domänenmodell entlogiken & Typen
  stärken (Abschnitt 5; umgesetzt).
- [`06-state-manager.md`](./06-state-manager.md) — `state_manager` aufräumen &
  Kapselung schärfen (Abschnitt 6; umgesetzt, J11 offen).
- [`07-commands.md`](./07-commands.md) — Commands entdoppeln, Slot-Move-Familie
  schneiden, Muster vereinheitlichen (Abschnitt 7; umgesetzt #50).
- [`08-input.md`](./08-input.md) — `input`-Modul aufräumen (Abschnitt 8; umgesetzt).
- [`09-naming.md`](./09-naming.md) — Namens- & Sprachkonsistenz (Abschnitt 9;
  umgesetzt #52).
- [`10-errors-and-methods.md`](./10-errors-and-methods.md) — CLI-Begriffe aus
  Lib-Fehlern verbannen (getippte Fehler + CLI-Hinweise, umgesetzt #53) und
  Methoden-Komplexität (umgesetzt mit #50) (Abschnitt 10).

---

## 12. Edit-Commands: Layout-Konsistenz & optionaler Rebuild

**Hintergrund.** Editier-Commands (`move`, `place`, `remove`, `combine`, `split`,
`unplace`) ändern die **Fotos** einer Seite, lassen die **Slots** von Auto-Seiten
aber bewusst veraltet — Slots sind ein Cache des letzten Solver-Laufs und werden
beim nächsten `build` neu berechnet (`page_change_detection.rs:49` überspringt
Manual-Seiten, alles andere wird bei count-/Struktur-Abweichung als *outdated*
neu gelöst). Die GUI versteckt das, weil sie nach jeder Mutation einen
inkrementellen Build (`BuildPlan::Auto`) fährt. Die CLI committet dagegen den
Zwischenstand `photos ≠ slots` direkt.

`check_validity` erzwingt die Gleichheit `photos.len() == slots.len()` daher nur
noch auf **Manual**-Seiten (dort sind Slots autoritativ); auf Auto-Seiten ist ein
Mismatch ein legaler „needs rebuild"-Zustand. Verwandt: §6 (`state_manager`,
Invarianten), §7.1 (`run_command`-Wrapper), §1 (`BuildPlan`).

### 12.1 Manual-Ziel beim `move` braucht einen Slot (Bug, offen)

Der Standard-Move pusht beim Ziel `DstMove::Page` Fotos ohne Slot
(`move_cmd/standard.rs:154` für die Slots-Variante, `:194` für die
Pages-Variante). Für Auto-Ziele ist das korrekt (der Rebuild regeneriert die
Slots). Ist das **Ziel aber eine Manual-Seite**, entsteht eine echte, dauerhafte
Inkonsistenz: Manual-Seiten werden nie neu gelöst, also bleibt `photos > slots`
bestehen — und `check_validity` meldet das zu Recht.

Der Befund gilt nach #50 unverändert; verifiziert mit `DstMove::Page(1)` auf eine
Manual-Seite → `photos=2, slots=1`. Der vorhandene Test
`move_slot_into_manual_page_creates_positioned_slot` deckt nur den expliziten
Pfad `DstMove::ManualAt` ab, nicht diesen.

**Fix:** Beim Verschieben auf eine Manual-Seite je Foto einen Slot erzeugen,
analog zu `apply_move_to_manual` (`move_cmd/manual.rs:84–85`), das Foto + Slot
bereits paarweise anhängt. Slot-Größe aus der Quell-Seite übernehmen bzw.
`default_manual_slot_size` (`move_cmd/manual.rs:111`) als Fallback. Alternativ den
Move auf ein Manual-Ziel über `DstMove::Page` ablehnen und auf `DstMove::ManualAt`
verweisen. Empfehlung: Slot erzeugen (deckt CLI- und GUI-Pfad gleich ab).

### 12.2 CLI-Rückmeldung: welche Seiten einen Rebuild brauchen (offen)

Nach einem Edit sieht der CLI-Nutzer nicht deutlich, dass betroffene Seiten neu
gebaut werden müssen. Die Information liegt vor — `outdated_pages_indices()`
(`state_manager.rs:178`) liefert sie, `status` nutzt sie —, ist aber zu
unauffällig.

**Vorschlag:**
- Editier-Commands geben die betroffenen Seiten im Result zurück und die CLI
  hängt einen Hinweis an (z. B. „pages 2,3 need rebuild — run `fotobuch build`";
  Hinweistext oberflächenspezifisch, siehe §10).
- `status` hebt outdated-Seiten klar hervor (eigene Zeile/Markierung statt nur im
  Detail).
- In der Nutzer-Doku (`docs/book`) erklären: Auto-Slots sind ein Cache; nach
  einem Edit ist ein `build` nötig, damit Layout/PDF stimmen.

### 12.3 Idee: `move` + `rebuild` in einem (Entscheidung offen)

> **Status: noch nicht entschieden.** Ob und wie das umgesetzt wird, muss der
> Eigentümer abnehmen. Dieser Abschnitt sammelt nur Optionen.

Heute zwei Schritte (Edit, dann `build`); die GUI verbindet beides automatisch.
Optionen, um das auch in der CLI zu bekommen:

- **A — Flag pro Command** (`page move … --build`): einfach, aber Flag + Build-
  Verdrahtung dupliziert sich über ~6 Commands.
- **B — gemeinsame Orchestrierungs-Stufe:** Edit-Command liefert „betroffene
  Seiten", ein Wrapper fährt danach optional `BuildPlan::Auto { pages }`. Das ist
  der `run_command`-Wrapper aus §7.1 — eine Stelle, konsistent für alle Edits.
- **C — Config-Flag** (z. B. `config.preview.autobuild_after_edit`): wird nach
  jeder Mutation abgefragt; ist es gesetzt, baut die CLI automatisch (GUI-Verhalten
  als Default), `--no-build` zum einmaligen Abschalten. Macht die persistierte YAML
  immer konsistent, kostet aber bei jedem Edit Zeit und erschwert Batch-Workflows
  (mehrere Moves, dann ein Build).

**Knackpunkt Commit-/Baseline-Semantik:** `find_last_build_state`
(`state_manager.rs:327`) erkennt den Build-Baseline nur am Commit-Präfix
`build:`/`rebuild:`. Ein kombinierter Commit `"page move …"` wäre **kein**
erkannter Baseline → `outdated_pages_indices()` vergliche gegen einen älteren
Build und hielte die gerade neu gelösten Seiten evtl. weiter für veraltet. Ein
kombinierter Schritt muss daher entweder **zwei Commits** erzeugen (Edit +
`build:`) oder eine build-erkennbare Message tragen, oder die Baseline-Erkennung
wird vom Präfix entkoppelt.

**Tendenz (zur Diskussion):** Option B oder C, Rebuild opt-in bzw. config-
gesteuert (Default aus, damit Batch-Edits möglich bleiben), mit zwei Commits.
Endgültige Entscheidung steht aus.
