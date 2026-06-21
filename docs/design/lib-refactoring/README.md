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
13. Edit-Commands: Layout-Konsistenz & optionaler Rebuild

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

**Status: offen.** Vollständiger Plan: [`04-solver.md`](./04-solver.md). Befunde
in Kürze:

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

## 5. Domänenmodell (`dto_models`)

**Status: offen** (Teil 5.2 Cover-Geometrie bereits erledigt, siehe
[`02-cover.md`](./02-cover.md)). Vollständiger Plan:
[`05-dto-models.md`](./05-dto-models.md). Befunde in Kürze:

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
- [`04-solver.md`](./04-solver.md) — Modul-/Namensstruktur des Solvers
  (Abschnitt 4; umgesetzt).
- [`05-dto-models.md`](./05-dto-models.md) — Domänenmodell entlogiken & Typen
  stärken (Abschnitt 5; offen).

Weitere Pläne (Commands …) folgen demselben Schema.

---

## 13. Edit-Commands: Layout-Konsistenz & optionaler Rebuild

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

### 13.1 Manual-Ziel beim `move` braucht einen Slot (Bug, umzusetzen)

`execute_move_to` pusht beim Ziel `DstMove::Page` Fotos ohne Slot
(`move_cmd.rs:188–190` für die Slots-Variante, `:225–227` für die Pages-Variante).
Für Auto-Ziele ist das korrekt (der Rebuild regeneriert die Slots). Ist das
**Ziel aber eine Manual-Seite**, entsteht eine echte, dauerhafte Inkonsistenz:
Manual-Seiten werden nie neu gelöst, also bleibt `photos > slots` bestehen — und
`check_validity` meldet das nach dem Fix nun zu Recht.

**Fix:** Beim Verschieben auf eine Manual-Seite je Foto einen Slot erzeugen,
analog zu `execute_move_to_manual` (`move_cmd.rs:316–326`), das Foto + Slot bereits
paarweise anhängt. Slot-Größe aus der Quell-Seite übernehmen bzw. `default_manual_
slot_size` (`move_cmd.rs:350`) als Fallback. Alternativ den Move auf ein
Manual-Ziel über `DstMove::Page` ablehnen und auf `DstMove::ManualAt` verweisen.
Empfehlung: Slot erzeugen (deckt CLI- und GUI-Pfad gleich ab).

### 13.2 CLI-Rückmeldung: welche Seiten einen Rebuild brauchen (umzusetzen)

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

### 13.3 Idee: `move` + `rebuild` in einem (Entscheidung offen)

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
