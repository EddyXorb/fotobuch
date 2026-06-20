# Lib-Refactoring — Strukturvorschläge für `src/`

> Status: teils umgesetzt. Abschnitte 1–3 sind im Kern erledigt (Details in den
> separaten Plänen `0x-*.md`, siehe Abschnitt 12); Abschnitte 4–11 sind offen.
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
10. Fehlermeldungen: CLI-Begriffe aus dem Lib-Code verbannen
11. Innerhalb von Methoden
12. Umsetzungspläne (separate Dokumente) → [`01-build.md`](./01-build.md)

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

**Status: Konsolidierung erledigt.** Die früher über `multipage_build` und
`rebuild_single_page` verstreute Cover-Logik (Fallunterscheidung *free vs.
structured*, `split_cover_photos`, `build_cover_page`, `update_cover_page`) liegt
jetzt gebündelt in `commands/build/build_layout/cover_page.rs`.

**Offen bleibt** die Cover-**Geometrie** (`spread_width_mm`, `spine_width_mm`,
`resolved_spine_text`), die weiterhin als Berechnungslogik auf dem Daten-DTO
`CoverConfig` (`cover_config.rs:144–173`) liegt statt in einem eigenen Wertobjekt
— dazu kommt eine tote, irreführende zweite `impl CanvasConfig for CoverConfig`
(`cover_config.rs:176`, ohne Consumer). Plan: [`02-cover.md`](./02-cover.md)
(verwandt mit Abschnitt 5.2).

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

### 4.1 Doppelbenennung (verletzt die `mod.rs`-Konvention)

CLAUDE.md verlangt: kein `mod.rs`, Modul = Ordnername, Inhalt in gleichnamiger
Datei daneben. Gebrochen durch **Modul-Inception**:

- `src/solver/solver.rs` ⇒ Pfad `solver::solver::run_solver`; explizit mit
  `#[allow(clippy::module_inception)]` (`solver.rs:17`) übergangen.
- `src/solver/ga_solver/solver.rs` ⇒ `solver::ga_solver::solver::…`.

Inhalt thematisch benennen: `solver/solver.rs` → `runner.rs`/`orchestrator.rs`
(oder direkt in `solver.rs` heben), `ga_solver/solver.rs` → `genetic_algorithm.rs`.

### 4.2 Benennungsschema mischt Algorithmus und Aufgabe

| Modul | Achse |
|---|---|
| `page_layout_solver`, `book_layout_solver`, `cover_solver` | Aufgabe |
| `ga_solver`, `bellman_solver`, `affine_solver` | Algorithmus |

`ga_solver` und `bellman_solver` sind **generische, domänenfreie Frameworks**
(`ga_solver.rs:12` "Zero Dependencies on domain-specific code"); `bellman_solver`
ist `pub(crate)`-Infrastruktur ohne Fotobuch-Bezug. Vorschlag: generische
Algorithmen in ein Subverzeichnis `solver/algorithms/` (oder `engine/`) und nach
Algorithmus benennen (`genetic_algorithm`, `bellman_dp`); aufgabenorientierte
Solver bleiben oben. `affine_solver` (unter `page_layout_solver/`) ist inkonsistent
zu seinen Siblings (`fitness`, `individual`, `tree`, `evolution` — alle nach
Domänenkonzept) → nach Aufgabe benennen.

### 4.3 `Request`-Struct leicht aufgebläht

`Request` (`solver.rs:25`) führt `config: &BookLayoutSolverConfig` **und**
`ga_config: &GaConfig` als Pflichtfelder; für `RequestType::SinglePage` wird
`config` nicht genutzt, ist aber Pflicht. Optionen: `RequestType` als Enum mit
variantenspezifischen Daten, oder die beiden Configs in einen
`SolverConfig`-Aggregat-Parameter bündeln.

### 4.4 `Params`-Alias stiftet Mehrfachnamen

`BookLayoutSolverConfig` wird 6× als `Params` re-importiert
(`book_layout_solver/model.rs:10` u. a.). Derselbe Typ heißt damit je nach Scope
`Params`, `BookLayoutSolverConfig` oder `config`. Einen Namen wählen.

### 4.5 Schicht-Trennung `data_models` vs. `dto_models`

`mod data_models;` ist `solver.rs:12` **nicht** `pub`, wird aber via
`prelude.rs:1` allen Solver-Untermodulen zugänglich gemacht — die fundamentale
Datenebene ist versteckt. Außerdem brechen Konvertierungsmethoden die Grenze:
`SolverPageLayout::to_layout_page()` (`data_models/layout.rs:223`) und
`Photo::from_photo_groups()` (`data_models/photo.rs:78`, enthält Sortier-Logik)
importieren aus `dto_models` — Abhängigkeit zeigt von innen nach außen.
Empfehlung: Konvertierung an *einer* Grenzschicht bündeln (z. B.
`solver/conversion.rs`), Datenmodelle logikfrei halten, Modul-Rollen in den
Modul-Docs benennen.

### 4.6 Weitere

- `page_layout_solver/tree.rs` (654 Z.) und `data_models/layout.rs` (694 Z.) zu
  groß; Bleed-Skalierung (`layout.rs:259–320`) ist ein eigener Algorithmus →
  eigenes File.
- `GAPageEvaluator` (Trait-Impl) lebt im Fassadenfile `book_layout_solver.rs:131`
  statt in einem Submodul.
- Hardcodierte Magic Numbers + TODOs in `run_ga` (`page_layout_solver.rs:44–65`:
  `tournament_size = 3`, `seed = 42`, "Add … to GaConfig").
- Inkonsistente Submodul-Sichtbarkeit (`ga_solver` deklariert `pub mod`,
  `page_layout_solver` `mod`).
- `ProjectConfig.page_layout_solver: GaConfig` (`project_config.rs:10`) wird auch
  im Multi-Page-Kontext genutzt → Feldname irreführend, eher `ga_config`.

---

## 5. Domänenmodell (`dto_models`)

### 5.1 Name `dto_models` irreführend

"DTO" suggeriert logiklose Transporthüllen; das Modul enthält aber Domänenlogik:
`ProjectState::check_validity()` (`state.rs:91–145`), `page_dimensions_mm()`,
`auto_page_indices()`, `BookLayoutSolverConfig::validate()`
(`book_layout_solver_config.rs:184–252`). Besser `models` / `domain`.

### 5.2 Logik liegt auf Config-/State-DTOs

- **`ProjectState` ist ein Gott-Objekt-Keim** (`state.rs`): Persistence (`load`/
  `save`, Datei-I/O), Validierung (`check_validity`) und Domain-Queries in einem
  Typ. Persistence aus dem Domänentyp herauslösen.
- **Cover-Geometrie** auf `CoverConfig` (`cover_config.rs:144–173`) → in ein
  eigenes Wertobjekt; ausgearbeitet im Plan [`02-cover.md`](./02-cover.md).
- **`BookLayoutSolverConfig::validate(total_photos)`** braucht Anwendungszustand
  (Fotoanzahl) → eigener Validator/Service statt Methode auf dem Config-DTO.
- **`CanvasConfig`-Trait** in `book_config.rs:31` definiert, aber einziger
  Konsument ist der Solver (`canvas.rs:48`) → näher an `solver` oder in ein
  `traits`-Modul.

### 5.3 Redundanz & schwache Typen

- **`aspect_ratio()` 3× identisch** (`photo_file.rs:31`, `canvas.rs:42`,
  `layout.rs:74`) → gemeinsames Trait/Helfer.
- **`LayoutPage.page` denormalisierte Index-Invariante** (`layout_page.rs:24`,
  muss `== layout[i]`-Index sein, wird von `renumber_pages`/`check_validity`
  bewacht). Wenn das Array kanonisch ordnet, ist das Feld redundanter Bug-Boden —
  Wegfall erwägen.
- **`PhotoGroup.sort_key: String`** (`photo_group.rs:11`) ist faktisch immer ein
  ISO-8601-Timestamp → `DateTime<Utc>`/`NaiveDate` (analog `PhotoFile.timestamp`),
  schwacher Name → `group_timestamp`.
- **`CoverMode`-Default-Falle:** Enum-`#[default]` ist `Free`
  (`cover_config.rs:15`), `CoverConfig::default().mode` aber `Split` (`:130`) —
  je nach Aufruf unterschiedliches Default. Vereinheitlichen.
- **Deprecated-Felder** im aktiven DTO (`book_layout_solver_config.rs:51–66`:
  `mip_rel_gap` u. a., "nur für Deserialisierung") → Migrationsschritt oder
  separater Legacy-Typ.
- **`BookConfig` enthält `cover`/`appendix` als Kinder** (`book_config.rs:7`) →
  vierstellige Zugriffspfade; flachere `ProjectConfig`-Struktur mit
  `Option<CoverConfig>` erwägen.

---

## 6. `state_manager`

- **`mgr.state: pub`** (`state_manager.rs:60`) öffnet die Kapselung — ~200
  Stellen greifen direkt auf `mgr.state.*` zu und können Invarianten (Numerierung,
  Validität) verletzen, ohne dass `StateManager` es bemerkt. Zugriff über Methoden
  kanalisieren (zumindest schreibend).
- **`renumber_pages` ist freie `pub`-Funktion** mit ungenutztem `_has_cover`
  (`state_manager.rs:29`, dead param) und wird von Commands direkt importiert
  (`remove.rs:252`, `build/plan.rs:72`) → als Methode auf `ProjectState`/
  `[LayoutPage]` kapseln, toten Parameter entfernen.
- **Zwei Wege zum State:** `StateManager::open` (mit Lifecycle/Auto-Commit) vs.
  freie `load_project_state()` (`state_manager.rs:390`, ohne Garantien) → kein
  Typschutz vor falscher Wahl. `open()` hat zudem unsichtbaren Seiteneffekt
  (`auto_commit_manual_edits`, `:125`), kaum dokumentiert.
- **`finish_always` Rückgabe inkonsistent:** liefert `Option<ProjectState>`,
  obwohl per Doku immer `Some` → `Result<ProjectState>` ohne `Option`.
- **Veralteter Doc-Kommentar:** `ensure_build_baseline` nennt Varianten
  `"Loaded"/"NoBuildCommit"`, real `LazyLoad::Failed` (`:289`).
- **Sichtbarkeit in `state_diff.rs`** uneinheitlich (`pub` statt `pub(super)` für
  rein interne Helfer).
- `page_change_detection.rs` (1025 Z.) ist v. a. Tests (~770) — ok, aber
  Test-Fixtures extrahierbar.

---

## 7. Commands: Duplizierung, Struktur, Config/Result-Muster

### 7.1 Duplizierte Logik (höchste Priorität)

- **3× „unplaced finden"** mit gleichem Kern, drei Signaturen:
  `remove.rs:184 collect_unplaced_ids`, `place.rs:57 find_unplaced`,
  `status.rs:82 count_unplaced` → eine Query-Funktion.
- **2× „leere Seiten entfernen":** `remove.rs:146 remove_empty_pages` (privat,
  ohne Rückgabe) dupliziert `page/helpers.rs:74 delete_empty_pages` (`pub(crate)`,
  bereits von `unplace.rs` genutzt) → `remove` soll die vorhandene importieren.
- **2× Page-Listen-Formatter:** `place.rs:322 format_page_list` (`usize`,
  ", ") vs. `page/helpers.rs:201 format_pages_list` (`u32`, ",") →
  zusammenführen (auch Trenner vereinheitlichen).
- **StateManager open/finish + „nothing to do"-Frühreturn** an ≥8 Stellen
  (`place.rs:127,149`, `remove.rs:234`, `unplace.rs:26`, `move_cmd.rs:113,279`)
  → `run_command`-Wrapper.

### 7.2 Zu große Dateien / vermischte Verantwortung

- **`move_cmd.rs` (1096 Z.)** enthält drei orthogonale Operationen: `execute_move_to`,
  `execute_move_to_manual`, `execute_swap` → `swap_cmd.rs` abspalten (analog zu
  `split.rs`/`combine.rs`); Layout-Rechnung `adapt_manual_slot_ratios`/
  `photo_pixel_size` (`:358–393`) in eigenes Modul.
- **`place.rs` (626 Z.)** mischt Orchestrierung mit dem chronologischen
  Algorithmus (`compute_page_ranges`/`find_target_page`/`place_chronologically`,
  `:207–283`) → Algorithmus in seiteneffektfreies `page_placement.rs` ziehen
  (isoliert testbar).
- **`remove.rs` (652 Z.)** dispatcht vier Operationen über `RemoveTarget`.

### 7.3 Datei-vs-Ordner inkonsistent

`add.rs` (280 Z.) hat bereits Submodule `add/deduplication.rs`, `add/merge.rs`,
ist also de facto Ordnermodul; `remove.rs`/`place.rs` (>600 Z.) sind Einzeldateien,
`config.rs` (218 Z.) hat einen Ordner. Kriterium festlegen (z. B. „Ordner ab
Submodul/Größe X") und konsistent anwenden.

### 7.4 Config/Result-Muster uneinheitlich

`AddConfig/AddResult` etc. ok, aber: `StatusReport` statt `…Result`
(`status.rs:57`); kein Config-Struct bei `history`/`undo`/`unplace`;
`unplace` gibt fremdes `PageMoveResult`; `page weight` gibt `()`; `PageMoveCmd`
ist Config-**Enum** statt -Struct. Muster bewusst definieren (nach Abschnitt 1
lösen Plan-Enums die Config-Structs für build/rebuild ohnehin ab — das als
gewollte Ausnahme dokumentieren).

### 7.5 Namenskollisionen

- **`SlotInfo` 2×** öffentlich im `commands`-Namespace: `status.rs:32` vs.
  `page/types.rs:250` → `StatusSlotInfo` / `PageSlotInfo`.
- **`ProjectState_`** (Trailing-Underscore, `status.rs:21`) ist ein Enum
  `{Empty, Clean, Modified}` — umbenennen zu `ProjectStatus`/`BookStatus`.
- **`config()`** heißt wie Modul *und* Typname.

---

## 8. `input`-Modul

- **Zwei `metadata.rs` auf verschiedenen Ebenen:** `input/metadata.rs` (nur
  `compute_partial_hash`, Duplikat-Hashing) vs. `input/scan/metadata.rs`
  (`enrich_photo_metadata`, EXIF/Orientierung). Nur über Pfad unterscheidbar →
  `input/metadata.rs` → `input/hashing.rs`.
- **`enrich_photo_metadata` als Teil der Scan-API exportiert**
  (`scan.rs:16`), ist aber konzeptuell kein Scanner-Output, sondern ein
  Enricher → Exportpfad korrigieren.
- **Kommentar-Leiche** `// pub mod grouper; // Will be added in Commit 5`
  (`input.rs:6`) entfernen.

---

## 9. Querschnitt: Namens- & Sprachkonsistenz

- **Deutsch/Englisch gemischt im Code** (Bezeichner englisch, Kommentare
  teils deutsch), konzentriert in `commands/remove.rs` (`:50,89,101,117,145,
  150–152,166,249`), `state_manager.rs:25–28`, `commands/page/pos.rs:1`,
  `commands/build/helpers.rs:46,74`, `output/render.rs:47`. Auf Englisch
  vereinheitlichen (Prosa-Doku darf deutsch bleiben — aber eine Sprache pro
  Schicht). Typo `nothting` in `ga_solver/evolution.rs:16`.
- **Verb-Konvention der Command-Einstiege uneinheitlich:** bare verb
  (`add`, `remove`, `place`, `status`) vs. `execute_*` (alle `page/`-Commands,
  `unplace`) vs. `project_*`. Eine Konvention wählen (Empfehlung: bare verb).
- **`Result` vs. `Report`** (Abschnitt 7.4), **`data_models` vs. `dto_models`**
  (4.5/5.1), **`Params`-Alias** (4.4): jeweils einen Begriff pro Konzept.
- **Cache-Verben** uneinheitlich: `ensure_previews` vs. `build_final_cache` vs.
  `update_preview_cache`/`update_preview_pdf` → ein Verb pro Bedeutung.

---

## 10. Fehlermeldungen: CLI-Begriffe aus dem Lib-Code verbannen

### Befund

Mehrere Laufzeitfehler in der Lib nennen konkrete CLI-Kommandos und -Flags:

- `build/build_layout.rs:57,87` / `place.rs:101` „Run `fotobuch build` first."
- `build/build_layout.rs:70` „`page mode {} a`" und `:123,128` „`rebuild --page 0`"
- `build/plan.rs:120` „`fotobuch build release --force`"
- `page/types.rs:176` „`page mode {p} m`"
- `project/switch.rs:35` „`fotobuch project list`"

Zwei Probleme:

1. **Falscher Kontext für Nicht-CLI-Konsumenten.** Die GUI nutzt dieselbe Lib;
   ein GUI-Nutzer bekommt „Run `fotobuch build` first" — sinnlos.
2. **Drift.** Flag-/Kommandonamen (`--flex`, `--page`, `release --force`) liegen
   als String-Literale weit weg von den clap-Definitionen. Wird ein Flag
   umbenannt, bleiben die Strings stehen; nichts erzwingt Konsistenz.

### Grundidee: Bedingung und Abhilfe trennen

Eine Fehlermeldung wie „Run `fotobuch build` first" vermischt **zwei** Dinge:

- **die Bedingung** — *was* ist schiefgelaufen? („es gibt noch kein Layout")
- **die Abhilfe** — *wie* behebt der Nutzer es? („führe `fotobuch build` aus")

Die Bedingung ist allgemeingültig und gehört in die Lib. Die Abhilfe ist
oberflächenspezifisch: ein CLI-Nutzer soll einen Befehl tippen, ein GUI-Nutzer
auf einen Knopf klicken. Heute steht beides als ein fixer String in der Lib —
deshalb ist er für die GUI falsch und enthält CLI-Wissen, das die Lib nichts
angeht.

**Regel:** Die Lib beschreibt nur die Bedingung. Die jeweilige Oberfläche (CLI,
GUI) ergänzt die Abhilfe.

### Schritt 1 — Lib: getippte, CLI-freie Fehler

Statt `anyhow::bail!("… Run \`fotobuch build\` first")` gibt die Lib einen
*getippten* Fehler zurück (`thiserror`-Enum je Command oder gemeinsam):

```rust
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("no layout exists yet")]
    NoLayout,
    #[error("layout has uncommitted changes on pages {pages:?}")]
    LayoutDirty { pages: Vec<usize> },
    #[error("page {idx} is in manual mode")]
    PageIsManual { idx: usize },
    #[error("page selection is not allowed for a release build")]
    PagesWithRelease,
}
```

Jede Variante ist eine **maschinenlesbare** Fehlerursache; der `#[error(...)]`-Text
nennt nur den Sachverhalt, keine Befehle. Die GUI kann diesen Text direkt
anzeigen und zusätzlich anhand der Variante einen eigenen Knopf vorschlagen.

### Schritt 2 — CLI: Abhilfe mit echten Flags anhängen

Die CLI fängt den getippten Fehler ab und hängt einen Hinweis an. Beispiel-Ablauf
für `BuildError::NoLayout`:

```text
Lib gibt zurück:   Err(BuildError::NoLayout)
CLI zeigt:         error: no layout exists yet
                   hint:  run `fotobuch build` first
GUI zeigt:         "No layout exists yet."  + [Build]-Knopf
```

Die Übersetzung Variante → Hinweis lebt ausschließlich in der CLI-Kiste:

```rust
// nur in der CLI-Kiste, direkt neben den clap-Definitionen
fn hint(err: &BuildError) -> String {
    match err {
        BuildError::NoLayout       => format!("run `{BUILD}` first"),
        BuildError::LayoutDirty{..} => format!("run `{BUILD}` or `{BUILD} {RELEASE} --{FORCE}`"),
        BuildError::PagesWithRelease => format!("`--{PAGES}` cannot be combined with `{RELEASE}`"),
        BuildError::PageIsManual{idx} => format!("switch it first: `{PAGE} mode {idx} a`"),
    }
}
```

Die Bezeichner `BUILD`/`PAGE`/… sind hier **keine** freien Literale: sie werden
zur Laufzeit aus clap gelesen (siehe Mechanismus 2), damit der Hinweis denselben
Namen zeigt wie Help und Parser.

### Das Drift-Problem genau benannt

„Drift" heißt: Hinweistext und tatsächliche CLI laufen auseinander, **ohne dass
es jemand merkt.** Es gibt zwei Spielarten:

- **Spielart A — falscher Name.** Jemand benennt das Flag `--page` in `--single`
  um (in der clap-Definition). Der hartkodierte String „`rebuild --page 0`" bleibt
  stehen. Ergebnis: die Meldung empfiehlt ein Flag, das es nicht mehr gibt. Kein
  Compilerfehler, kein Testfehler — niemand merkt es.
- **Spielart B — fehlender Hinweis.** Jemand fügt eine neue Fehlerbedingung
  hinzu (`BuildError::CoverLocked`), vergisst aber, ihr eine Meldung zu geben. Der
  Nutzer bekommt im besten Fall einen nichtssagenden Standardtext.

### Lösung gegen Drift — zwei Mechanismen

**Mechanismus 1 gegen Spielart B: exhaustives `match` ohne `_`-Arm.**
Die `hint`-Funktion oben behandelt jede Variante einzeln und hat **keinen**
Auffang-Arm (`_ => …`). Fügt jemand `BuildError::CoverLocked` hinzu, ist das
`match` nicht mehr vollständig → **die CLI kompiliert nicht**, bis der Hinweis
ergänzt ist. Der Compiler erzwingt, dass jede Fehlerursache eine
Nutzer-Abhilfe bekommt.

**Mechanismus 2 gegen Spielart A: clap bleibt die einzige Quelle — Namen werden
*aus* clap gelesen, nicht hineingegeben.**

Wichtig vorweg: An clap ändert sich **nichts**. Die Flags bleiben per Derive
definiert (`#[arg(long)]`), und **Help wird unverändert aus genau diesen echten
Definitionen erzeugt.** Wir füttern clap *nicht* mit Konstanten — das wäre genau
die schlechte Variante, bei der Help von der Definition abkoppelt.

Stattdessen holt sich der Hinweis den anzuzeigenden Namen zur Laufzeit aus dem
bereits gebauten Command-Baum (clap-Derive liefert `Cli::command()`):

```rust
/// Liefert das echte `--long` eines Args (per Arg-Id) aus dem clap-Baum.
/// Findet es das Arg nicht, schlägt es fehl — wird vom Test unten erkannt.
fn long(cmd: &clap::Command, sub_path: &[&str], arg_id: &str) -> String {
    let sub = sub_path.iter().fold(cmd, |c, name| {
        c.find_subcommand(name).expect("unbekanntes Subcommand")
    });
    let arg = sub
        .get_arguments()
        .find(|a| a.get_id() == arg_id)
        .expect("unbekannte Arg-Id");
    format!("--{}", arg.get_long().expect("Arg hat kein --long"))
}
```

Der angezeigte Name stammt damit buchstäblich aus clap und kann gar nicht von
Help/Parser abweichen. Im `hint()`-Code steht nur noch die **Id** (z. B.
`"force"`), nie der sichtbare Name:

```rust
let cmd = Cli::command();
let force = long(&cmd, &["build"], "force");   // ergibt "--force" laut clap
format!("run `fotobuch build {force}`")
```

Bei clap-Derive ist die Arg-Id standardmäßig der Feldname. Wird das Feld
umbenannt, passt die Id `"force"` nicht mehr → `long(...)` findet nichts.

**Absicherung per Test:** damit dieser Fehlschlag nicht erst beim Nutzer
auftritt, prüft ein Test alle vom Hinweis benutzten Lookups gegen den echten
clap-Baum:

```rust
#[test]
fn hint_lookups_resolve() {
    let cmd = Cli::command();
    long(&cmd, &["build"], "force");   // schlägt fehl, falls entfernt/umbenannt
    long(&cmd, &["build"], "pages");
    // … je benutztem Lookup eine Zeile
}
```

Optional die Lookup-Liste (`(sub_path, arg_id)`) einmal zentral halten und sowohl
von `hint()` als auch vom Test durchlaufen — dann kann auch diese Liste nicht
auseinanderlaufen. So bleibt clap die alleinige Namensquelle, Help kommt
unverändert aus den echten Flags, und der Hinweis spiegelt sie nur wider.

### Zusammengefasst

- **Lib:** sagt nur, *was* falsch ist (getippter Fehler, kein CLI-Text).
- **CLI:** sagt, *wie* man es behebt (Hinweis mit echten Flags).
- **Spielart B** (fehlender Hinweis) fängt das exhaustive `match` zur Compilezeit.
- **Spielart A** (falscher Name): clap bleibt einzige Quelle — der Hinweis liest
  den Namen aus clap heraus; ein Test stellt sicher, dass jeder Lookup auflöst.

### Umfang

Betrifft v. a. `commands/`: die `bail!`/`anyhow!`-Stellen aus dem Befund auf
getippte Fehler umstellen; Next-Steps-Ausgaben wie `project/new.rs:78–81` und
`switch.rs:35` aus der Lib in die CLI verschieben. Doc-Comments (`//! \`fotobuch
…\``) dürfen bleiben — sie sind Dokumentation, keine Laufzeit-Strings.

---

## 11. Innerhalb von Methoden

Die früheren build-Punkte (`multipage_build`-Zweige, `RenderContext`,
`incremental_build`, `apply_page_filter`) sind mit Abschnitt 1 **erledigt**.
Offen bleibt:

- **`execute_move_to`** (`move_cmd.rs:33–248`) hat 6 Frühreturns über mehrere
  Abstraktionsebenen → Validierung/Dispatch/Mutation trennen (gehört zum noch
  offenen Commands-Komplex, Abschnitt 7.2).

---

## 12. Umsetzungspläne (separate Dokumente)

Die konkreten, in Conventional Commits gegliederten Umsetzungspläne liegen je
Themenblock in eigenen Dateien neben diesem Dokument:

- [`01-build.md`](./01-build.md) — Vereinheitlichung der build-/rebuild-Methoden
  (Abschnitte 1–3).
- [`02-cover.md`](./02-cover.md) — Cover-Geometrie aus dem DTO lösen
  (Abschnitt 2 / 5.2; Konsolidierung bereits erledigt).
- [`03-build-structure.md`](./03-build-structure.md) — Benennung & Kohäsion im
  build-Pfad (Abschnitt 3; Strukturbefunde bereits erledigt).

Weitere Pläne (Solver, `dto_models`, Commands …) folgen demselben Schema.
