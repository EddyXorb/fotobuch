# Umsetzungsplan 1 — Vereinheitlichung der build-/rebuild-Methoden

> Konkretisiert die Abschnitte 1–3 des [Hauptdokuments](./README.md).
> Inkrementell in Conventional Commits, jeder Schritt einzeln reviewbar.

## Ziel in einem Satz

Statt zweier CLI-Kommandos (`build`, `rebuild`) mit zusammen sechs
Orchestrierungsfunktionen, die denselben Ablauf jeweils neu zusammenbauen, soll
es **eine** Pipeline-Funktion geben, die den festen Ablauf kennt, und **pro
Situation einen kleinen Handler**, der nur den variablen Teil liefert.

## Ausgangslage: die heutigen Bausteine

Damit der Plan ohne Blick in den Code lesbar ist, hier die Stellen, um die es
geht (alle unter `src/commands/`):

| Heute                       | Datei                              | Rolle |
|-----------------------------|------------------------------------|-------|
| `build()`                   | `build.rs`                         | CLI-Einstieg; verzweigt zu first/incremental/release |
| `rebuild()`                 | `rebuild.rs`                       | CLI-Einstieg; verzweigt zu single/range/all |
| `first_build()`             | `build/first_build.rs`             | ganzes Buch neu lösen |
| `incremental_build()`       | `build/incremental_build.rs`       | nur geänderte Seiten neu lösen |
| `release_build()`           | `build/release_build.rs`           | finale Hochauflösungs-PDF, **kein** Neulösen |
| `multipage_build()`         | `build/core/multipage_build.rs`    | gemeinsamer Löser für ganzes Buch / Bereich |
| `rebuild_single_page()`     | `build/core/rebuild_single_page.rs`| eine Seite neu lösen (Cover oder Innenseite) |
| `MultiPageParams`           | `build/core/multipage_build.rs`    | 10-Feld-Parameterobjekt für `multipage_build` |

Wiederkehrende Schritte, die heute in mehreren dieser Funktionen kopiert sind:

1. **Bild-Cache** auffrischen (`preview`-Cache; bei Release stattdessen der
   hochauflösende `final`-Cache).
2. **Layout neu berechnen** (Solver-Aufruf) — der einzige Teil, der je nach
   Situation wirklich anders ist.
3. **`renumber_pages`** — setzt `layout[i].page = i` wieder gerade.
4. **Commit** — `StateManager::finish` (nur committen, wenn sich etwas änderte)
   bzw. `finish_always` (immer committen, für rebuild/release).
5. **PDF erzeugen** — `compile_preview` bzw. `compile_final`; bei `skip_pdf` nur
   den Pfad zurückgeben. Dieses `if skip_pdf { … } else { … }` steht 4×.
6. **`BuildResult`** zusammenbauen.

## Zielbild

**Schicht 1 — eine Pipeline.** Eine Funktion führt die Schritte 1–6 aus:

```text
run_build(mgr, project_root, plan: BuildPlan, opts: BuildOptions)
    -> CommandOutput<BuildResult>
```

**Schicht 2 — ein Handler je Situation,** gesteuert durch ein Enum, über das
`run_build` genau einmal verzweigt:

```text
enum BuildPlan {
    Full,                          // erster Build / rebuild ohne Argument
    Incremental { pages: Option<Vec<usize>> },
    Page(usize),                   // rebuild --page
    Range { start, end, flex },    // rebuild --range
    Release,                       // build --release (kein Neulösen)
}
```

Jeder Handler macht nur Schritt 2 und meldet, welche Seiten betroffen sind.
Die Unterschiede Cache-Art, Commit-Art und PDF-Art werden **Daten im Plan**,
keine eigenen Funktionen mehr.

---

## Commit-Plan

Regeln für jeden Commit: `cargo build`, `clippy --fix`, `cargo fmt` ohne
Warnungen; bestehende Tests grün. Die Phasen A und B ändern **kein Verhalten** —
gleiche Layouts und PDF-Pfade wie vorher; sie bauen nur die Teile, auf denen C
aufsetzt.

### Phase A — Bausteine (verhaltensneutral)

**A1 · `refactor(build): bundle skip flags into BuildOptions`**
Heute schleppen fünf Funktionen dasselbe Quartett `(mgr, project_root, skip_pdf,
skip_cache_update)` mit. → Ein `struct BuildOptions { skip_pdf,
skip_cache_update }` bündelt die zwei Flags.
*Verify:* kompiliert, Tests grün, keine Logikänderung.

**A2 · `refactor(build): extract render_pdf helper`**
Das 4× kopierte Muster „bei `skip_pdf` nur Pfad bauen, sonst kompilieren"
(`multipage_build.rs:140`, `incremental_build.rs:36,88`, `rebuild.rs:159`,
`release_build.rs:98`) → eine Funktion `render_pdf(...)`. Preview vs. final
unterscheidet ein `enum PdfTarget`.
*Verify:* PDF-Pfade in Tests unverändert.

**A3 · `refactor(build): capture RenderContext before mgr is consumed`**
`finish`/`finish_always` *verbrauchen* den `StateManager`, deshalb sichern heute
mehrere Funktionen vorher manuell `project_name` und `bleed_mm` in lokale
Variablen. → Ein `struct RenderContext { project_name, bleed_mm }`, einmal vor
dem Commit erzeugt.
*Verify:* Tests grün.

**A4 · `refactor(build): replace always_commit bool with CommitMode`**
Das Flag `always_commit: bool` entscheidet zwischen `finish` und
`finish_always`. → `enum CommitMode { Auto, Always }` macht die Absicht lesbar.
*Verify:* welche Pfade immer committen, bleibt identisch.

### Phase B — Cover-Logik bündeln

Hintergrund: Eine Cover-Seite kann *structured* (fester Slot-Aufbau, vom
deterministischen `cover_solver`) oder *free* (frei via GA-Solver) sein. Diese
Fallunterscheidung steht heute an **vier** Stellen verstreut — dreimal in
`multipage_build` (Cover-Fotos abspalten, Cover-Seite bauen, Free-Cover
nachträglich neu lösen) und in den `rebuild_cover_*`-Funktionen von
`rebuild_single_page`.

**B1 · `refactor(build): extract cover module`**
Neues `build/cover.rs` mit *einer* Funktion „erzeuge/aktualisiere Cover-Seite",
die intern in `free`/`structured` verzweigt; alle vier Altstellen rufen sie auf.
*Verify:* Cover-Tests (Split / Free / structured) grün, identische Slots.

### Phase C — Pipeline & Plan (der eigentliche Umbau)

**C1 · `refactor(build): introduce BuildPlan enum + situation handlers`**
Das `BuildPlan`-Enum (siehe Zielbild) plus je Variante eine **reine** Handler-
Funktion, die nur das Layout ändert und die betroffenen Seitenindizes liefert
(`Release` ändert nichts). Wird vorerst noch von den alten Orchestratoren
aufgerufen.
*Verify:* Unit-Test je Variante.

**C2 · `feat(build): add run_build pipeline`**
`run_build` führt die Schritte 1–6 aus und ruft für Schritt 2 den passenden
Handler. Läuft zunächst **parallel** zu den Altpfaden; ein erster Aufrufer
(`first_build`) wird umgestellt.
*Verify:* `first_build`-Tests grün gegen die neue Pipeline.

**C3 · `refactor(build): route build()/rebuild() through run_build`**
`build()` baut `Full | Incremental | Release`, `rebuild()` baut
`Full | Range | Page`; beide rufen nur noch `run_build`. Die heutigen
Vorab-Prüfungen (`validate_scope`, `skip_cover_if_needed`) wandern in die
Plan-Konstruktion.
*Verify:* gesamter build/rebuild/release-Testsatz grün.

### Phase D — Aufräumen

**D1 · `refactor(build): remove obsolete orchestrators`**
Löschen: `first_build`, `incremental_build`, `release_build`, `rebuild_all`,
`rebuild_range`, `rebuild_single`, `MultiPageParams` — ihre Felder sind in
`BuildPlan`/`BuildOptions`/`CommitMode` aufgegangen.
*Verify:* keine toten Imports; Tests grün.

**D2 · `refactor(build): slim BuildResult`**
`pages_swapped` (immer leer) und `total_cost` (immer 0.0) entfernen oder als
`Option`; `images_processed` an genau einer Stelle in `run_build` setzen.
*Verify:* CLI-Ausgabe / `print_build_result` angepasst.

**D3 · `refactor(build): hoist shared engine out of build/core`**
Die geteilte Engine (`run_build`, Handler, cover) nach `commands/build_engine.rs`;
`build.rs`/`rebuild.rs` werden dünne CLI-Adapter, `build/core.rs` entfällt.
Außerdem `build_photo_index` in ein neutrales Query-Modul ziehen, damit die
Lese-Kommandos `place`/`status` nicht mehr aus `commands::build` importieren.
*Verify:* `place`/`status` ohne `commands::build`-Import; Tests grün.

---

## Reihenfolge & Risiko

- **A vor B vor C:** erst kleine, verhaltensneutrale Schritte; die Plan-Handler
  (C1) sollen bereits die konsolidierte Cover-Funktion (B1) nutzen.
- **Höchstes Regressionsrisiko:**
  - *Manual-Pages* — Seiten im Manual-Modus dürfen vom Solver nicht
    umverteilt werden; sie werden vor dem Lösen herausgenommen und danach an
    alter Position wieder eingefügt (`extract_manual_pages`).
  - *Free-Cover-Nachlösung* — eine Free-Cover-Seite wird mit den korrekten
    Spread-Maßen separat nachgelöst.
  Beide in C1/C2 mit gezielten Tests absichern, **bevor** D1 die Altpfade löscht.
- **D erst**, wenn der komplette build/rebuild/release-Testsatz über `run_build`
  grün ist.
