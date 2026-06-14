# Lib-Refactoring — Strukturvorschläge für `src/`

> Status: Vorschlagssammlung, noch nicht umgesetzt. Reihenfolge = Priorität
> (oben = größte strukturelle Hebelwirkung). Kein fertiger Code, nur Ideen +
> Signatur-Skizzen.

Leitprinzipien für alle Vorschläge:

- **Eine Sache pro Abstraktionsebene.** Eine Methode orchestriert *oder* sie
  rechnet — nicht beides.
- **Konsistente Sprache.** Bezeichner + Doc-Comments durchgehend Englisch,
  Domänenbegriffe exakt wie in `docs/book/src/glossary.md`.
- **Konsistente Namensschemata.** Gleiche Rolle ⇒ gleiches Suffix/Präfix.

---

## 1. Kernproblem: Vielfalt der build-Methoden

### 1.1 Bestandsaufnahme

Aktuell existieren **zwei Einstiegskommandos** mit je eigener Verzweigung und
**sechs Orchestrierungsfunktionen**, die sich denselben Ablauf teilweise teilen,
teilweise duplizieren:

| Einstieg            | verzweigt zu          | ruft                  |
|---------------------|-----------------------|-----------------------|
| `build()`           | `release_build`       | `final_cache` + pdf   |
|                     | `first_build`         | `multipage_build`     |
|                     | `incremental_build`   | `rebuild_single_page`×N |
| `rebuild()`         | `rebuild_single`      | `rebuild_single_page` |
|                     | `rebuild_range`       | `multipage_build`     |
|                     | `rebuild_all`         | `multipage_build`     |

Daraus folgen die spürbaren Symptome:

- **Geteilter Ablauf, mehrfach kopiert.** Die Invariante
  *Cache sicherstellen → Solver/Layout ändern → renumber → commit → PDF
  kompilieren → `BuildResult` bauen* steht in `multipage_build`,
  `incremental_build`, `rebuild_single` und `release_build` jeweils erneut.
- **Aufgeblähte Signatur.** `MultiPageParams` hat 10 Felder
  (`multipage_build.rs:17`), davon mehrere ableitbar (`images_processed`,
  `always_commit`) oder Strategie-Detail (`commit_message`).
- **Wiederkehrendes Parameter-Quartett.** `(mgr, project_root, skip_pdf,
  skip_cache_update)` zieht sich durch `first_build`, `incremental_build`,
  `rebuild_single`, `rebuild_range`, `rebuild_all`.
- **Uneinheitliche Namen.** `first_build` / `incremental_build` /
  `release_build` (Suffix) vs. `rebuild_all` / `rebuild_range` /
  `rebuild_single` (Präfix) vs. `multipage_build` (nach Solver-Interna benannt).
- **Dupliziertes Mikro-Muster.** `if skip_pdf { root.join("{}.pdf") } else {
  compile_… }` steht 4× (`incremental_build.rs:36,88`,
  `multipage_build.rs:140`, `rebuild.rs:159`).

### 1.2 Zielbild: eine Einstiegsmethode, eine Ebene tiefer pro Situation verzweigen

Trennung in **zwei Schichten**:

**Schicht 1 — Orchestrierung (invariant, genau *eine* Funktion):**

```text
run_build(mgr, project_root, plan: BuildPlan, opts: BuildOptions)
    -> CommandOutput<BuildResult>
```

führt den festen Ablauf aus und delegiert nur den variablen Schritt
("welche Seiten entstehen neu?") an Schicht 2.

**Schicht 2 — Situations-Handler (je Situationstyp eine Funktion).** Modelliert
als Enum, über das `run_build` *einmal* `match`t:

```text
enum BuildPlan {
    Full,                         // erster Build / rebuild ohne Argument
    Incremental { pages: Option<Vec<usize>> },
    Page(usize),                  // rebuild --page
    Range { start, end, flex },   // rebuild --range
    Release,                      // build --release
}
```

Jeder Arm produziert nur die Layout-Änderung (bzw. `Release`: keine) und meldet
zurück, *welche* Seiten betroffen sind. Damit wird die Verzweigung genau dort
sichtbar, wo der Nutzer sie sich wünscht: eine Ebene unter dem Einstieg, ein Arm
pro Situation.

Effekt: `first_build`, `incremental_build`, `rebuild_all`, `rebuild_range`,
`rebuild_single` verschwinden als eigenständige Orchestratoren — sie werden zu
schlanken Plan-Konstruktoren. Die beiden CLI-Kommandos `build` und `rebuild`
bleiben getrennt (verschiedene UX), bauen aber beide nur einen `BuildPlan` und
rufen dasselbe `run_build`.

### 1.3 Variantenpunkte sauber modellieren

Die drei echten Unterschiede zwischen den Abläufen werden Daten statt
Code-Zweige:

- **Cache-Art:** Preview- vs. Final-Cache → ergibt sich aus `BuildPlan`
  (`Release` ⇒ final).
- **Commit-Art:** `finish` vs. `finish_always` → ein `commit: CommitMode` im Plan
  statt eines `always_commit: bool` in `MultiPageParams`.
- **PDF-Ziel:** `compile_preview` vs. `compile_final` vs. keins → ein einziger
  Helfer `render_pdf(plan, skip_pdf, …)`, der das oben genannte 4×-Muster
  aufsaugt.

### 1.4 `BuildResult` entschlacken

`pages_swapped` ist überall `vec![]`, `total_cost` überall `0.0` (TODO seit
`incremental_build.rs:81`). Beide Felder sind tote Schnittstelle.

- Entweder entfernen, bis sie real befüllt werden,
- oder ehrlich als `Option<…>` markieren.

`images_processed` hat je nach Pfad andere Semantik (mal `cache_result.created`,
mal `max(images_processed, …)`) — Semantik vereinheitlichen und an *einer* Stelle
(in `run_build`) setzen.

---

## 2. Cover-Logik konsolidieren

Cover-Behandlung ist über zwei Dateien und drei Stellen verstreut:

- `multipage_build.rs`: `split_cover_files` (oben), `build_cover_page` (Mitte),
  Free-Cover-Nachlösung (Schritt 7, `:121`).
- `rebuild_single_page.rs`: `rebuild_cover_free` / `rebuild_cover_structured`.

Beide Dateien kennen die Fallunterscheidung *free vs. structured* und rufen
`compute_cover_slots` / `warn_slot_count_mismatch`. `rebuild_single_page` ist
dabei das gute Vorbild: eine Verteilfunktion, die sauber in genau einen
Situations-Handler abzweigt (`cover_free` / `cover_structured` / `inner`).

Vorschlag: ein eigenes Modul `commands/build/cover.rs` (oder
`solver::cover_solver` erweitern), das die alleinige Antwort auf
"erzeuge/aktualisiere Cover-Seite" besitzt. `multipage_build` ruft es einmal auf,
statt Cover-Sonderfälle an drei Stellen einzustreuen.

---

## 3. Datei- & Modulstruktur des build-Pfads

- **`build/core/` ist ein Kopplungs-Smell.** `rebuild_single_page` und
  `multipage_build` liegen unter `build/`, werden aber vom Schwester-Kommando
  `rebuild.rs` über `super::build::{…}` mitbenutzt (`rebuild.rs:11`). Die
  faktische Abhängigkeitsrichtung sagt: das ist kein build-internes Detail,
  sondern eine **geteilte Build-Engine**. Vorschlag: gemeinsame Engine nach
  `commands/build_engine.rs` (oder einen lib-Modul `build/`) heben; `build.rs`
  und `rebuild.rs` werden zu dünnen CLI-Adaptern darüber.
- **`build/core.rs`** (`:1`) ist nur ein Re-Export-Knoten mit zwei Untermodulen —
  nach der Engine-Extraktion entfällt er.
- **Doc-Numerierung driftet vom Code ab.** In `rebuild_single` (`rebuild.rs:152`)
  heißt es "4. Compile Typst", danach kommt erst Schritt 5 (commit), dann die
  eigentliche Kompilierung. Solche Nummern-Kommentare bröckeln; nach der
  Pipeline-Vereinheitlichung sind sie überflüssig.

---

## 4. Namens- & Sprachkonsistenz (gesamte lib, leichter)

Diese Punkte sind kleiner, aber breit wirksam:

- **`Result` vs. `Report`.** Command-Ergebnisse heißen meist `…Result`
  (`AddResult`, `PlaceResult`, `BuildResult`), Status aber `StatusReport`
  (`commands.rs:51`). Einheitlich `…Result`.
- **`ProjectState_` mit Trailing-Underscore** (`commands.rs:51`) ist ein klarer
  Smell (Namenskollision umgangen). Umbenennen, z. B. `StatusProjectState` oder
  über Modulpfad disambiguieren.
- **Config/Result-Paare uneinheitlich.** `build`/`rebuild` brechen das
  `…Config`/`…Result`-Muster: `rebuild` nimmt `RebuildScope` (Enum) statt
  `RebuildConfig`. Nach Abschnitt 1 ohnehin durch `BuildPlan` ersetzt — dann
  bewusst dokumentieren, dass Plan-Enums die Config-Structs ablösen.
- **Datei-Doppelbenennung.** `solver/solver.rs` und `ga_solver/solver.rs`
  wiederholen den Ordnernamen. Per CLAUDE.md-Regel (kein `mod.rs`, Modul =
  Ordnername) gehört der Inhalt in `solver.rs` bzw. `ga_solver.rs` selbst, nicht
  in eine gleichnamige Datei darunter.
- **`data_models` vs. `dto_models`.** Es gibt `src/dto_models/` *und*
  `src/solver/data_models/`. Zwei "models"-Begriffe für verwandte Dinge stiften
  Verwirrung — Rollen klären und Begriffe trennen (z. B. `dto_models` →
  öffentliche API-Typen, `solver/data_models` → solver-interne Rechenmodelle —
  und das in den Modul-Docs benennen).
- **Cache-Verben uneinheitlich:** `ensure_previews` vs. `build_final_cache` vs.
  `update_preview_cache`/`update_preview_pdf` (helpers.rs). Ein Verb pro
  Bedeutung wählen (z. B. `ensure_*` für idempotentes Cache-Auffrischen).
- **Deutsch/Englisch gemischt im Code.** Doc-Comment und Rumpf von
  `collect_photos_as_groups` sind deutsch ("Sammelt alle Fotos…", "Nach
  Originalgruppe aufteilen", `helpers.rs:46–90`), `downsample` in `render.rs:47`
  ebenfalls. Auf Englisch vereinheitlichen (Prosa-Docs dürfen deutsch bleiben —
  aber dann projektweit konsequent eine Sprache pro Schicht).
- **Command-Module: Datei vs. Ordner gemischt.** `add.rs`/`remove.rs`/`place.rs`
  sind Einzeldateien, `build`/`page`/`project`/`config` Ordner. Kriterium
  festlegen (z. B. "Ordner ab N Untermodulen") und konsistent anwenden.

---

## 5. Innerhalb von Methoden (nach Abschnitt 1 großteils erledigt)

Falls Abschnitt 1 nicht sofort umgesetzt wird, lohnen diese lokalen Schnitte:

- **`multipage_build` Schritt 6** (`:101–116`): die Zweige *range* (splice) vs.
  *full* (assign) unterscheiden sich nur in einer Zeile + dem `pages_rebuilt`-
  Range. Zusammenführen.
- **`bleed_mm` / `project_name` „backup vor mgr-Verbrauch"** wird in 4 Funktionen
  manuell gesichert. In ein kleines `RenderContext`-Struct kapseln, einmal vor
  dem `finish` erzeugt.
- **`incremental_build` „nothing to do"-Zweig** (`:34–59`) dupliziert
  commit + pdf + Result-Aufbau des Erfolgspfads. Mit dem gemeinsamen
  Render-Helfer (1.3) verschmelzen.
- **`apply_page_filter`** (`incremental_build.rs:110`) ist ein Einzeiler-Wrapper
  um `retain` — inlinen oder als Methode auf der Pages-Liste.
