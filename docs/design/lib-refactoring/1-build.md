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

**Einstiegspunkte bleiben die CLI-Kommandos.** Konvention in `commands/`: jedes
CLI-Kommando hat eine gleichnamige Funktion. Das bleibt so — `build` *ist* der
Einstiegspunkt, `rebuild` ebenfalls (darf aber in `build.rs` wohnen, eine eigene
`rebuild.rs` ist nicht nötig). Beide tun nur zwei Dinge: aus ihren Argumenten
einen `BuildPlan` bauen und die gemeinsame Pipeline starten.

**Ein Plan je Situation** — das Enum, über das die Pipeline genau einmal
verzweigt:

```rust
enum BuildPlan {
    Full,                                            // erster Build / rebuild ohne Argument
    Incremental { pages: Option<Vec<usize>> },
    Page(usize),                                     // rebuild --page
    Range { start: usize, end: usize, flex: usize }, // rebuild --range
    Release,                                         // build --release (kein Neulösen)
}
```

**Eine Pipeline** kennt den festen Ablauf (Schritte 1–6 von oben). Sie ist
bewusst *kein* freistehendes `run_build`, sondern eine Methode auf dem Plan: so
liest sich der Aufruf als `plan.run(...)`, und der Name `build` bleibt dem
CLI-Kommando vorbehalten.

### Skizze

Die abstrakten Methoden rufen fast nur Submethoden auf — der Code liest sich als
kurze Geschichte:

```rust
// CLI-Einstieg: Kommando `build`
pub fn build(project_root: &Path, config: &BuildConfig)
    -> Result<CommandOutput<BuildResult>>
{
    let mgr = StateManager::open(project_root)?;
    let plan = BuildPlan::from_build_config(&mgr, config)?; // Full | Incremental | Release
    plan.run(mgr, project_root, config.into())
}

// CLI-Einstieg: Kommando `rebuild` (in build.rs, keine eigene Datei)
pub fn rebuild(project_root: &Path, scope: RebuildScope, opts: BuildOptions)
    -> Result<CommandOutput<BuildResult>>
{
    let mgr = StateManager::open(project_root)?;
    let plan = BuildPlan::from_rebuild_scope(&mgr, scope)?;  // Full | Range | Page
    plan.run(mgr, project_root, opts)
}

impl BuildPlan {
    /// Der feste Ablauf; nur `resolve_layout` unterscheidet sich je Situation.
    fn run(self, mut mgr: StateManager, project_root: &Path, opts: BuildOptions)
        -> Result<CommandOutput<BuildResult>>
    {
        refresh_image_cache(&mut mgr, &self, &opts)?;
        let changed_pages = self.resolve_layout(&mut mgr)?;
        renumber_pages(&mut mgr.state.layout, mgr.state.has_cover());

        let ctx = RenderContext::capture(&mgr);    // vor dem Verbrauch von mgr
        let changed_state = self.commit(mgr)?;     // finish / finish_always
        let pdf = render_pdf(&self, project_root, &ctx, opts.skip_pdf)?;

        Ok(build_result(&self, changed_pages, pdf, changed_state))
    }

    /// Der einzige situationsabhängige Schritt — eine Ebene tiefer, ein Arm je Typ.
    fn resolve_layout(&self, mgr: &mut StateManager) -> Result<Vec<usize>> {
        match self {
            BuildPlan::Full                       => solve_whole_book(mgr),
            BuildPlan::Incremental { pages }      => resolve_outdated_pages(mgr, pages.as_deref()),
            BuildPlan::Page(idx)                  => resolve_single_page(mgr, *idx),
            BuildPlan::Range { start, end, flex } => resolve_range(mgr, *start, *end, *flex),
            BuildPlan::Release                    => Ok(Vec::new()), // kein Neulösen
        }
    }
}
```

(Schematisch — Borrow-Details, etwa dass `commit` den `mgr` verbraucht, können in
der Umsetzung leicht abweichen.)

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

**C1 · `refactor(build): introduce BuildPlan enum + resolve_layout handlers`**
Das `BuildPlan`-Enum (siehe Zielbild) plus je Variante eine **reine**
`resolve_*`-Submethode, die nur das Layout ändert und die betroffenen
Seitenindizes liefert (`Release` ändert nichts). Wird vorerst noch von den alten
Orchestratoren aufgerufen.
*Verify:* Unit-Test je Variante.

**C2 · `feat(build): add BuildPlan::run pipeline`**
`BuildPlan::run` führt die Schritte 1–6 aus und ruft für Schritt 2 die passende
`resolve_*`-Submethode. Läuft zunächst **parallel** zu den Altpfaden; ein erster
Aufrufer (`first_build`) wird umgestellt.
*Verify:* `first_build`-Tests grün gegen die neue Pipeline.

**C3 · `refactor(build): route build()/rebuild() through BuildPlan::run`**
`build()` baut `Full | Incremental | Release`, `rebuild()` baut
`Full | Range | Page`; beide rufen nur noch `plan.run(...)`. Dabei zieht
`rebuild` aus `rebuild.rs` nach `build.rs` (Konvention: gleichnamige Funktion,
aber keine eigene Datei nötig); `rebuild.rs` wird gelöscht. Die heutigen
Vorab-Prüfungen (`validate_scope`, `skip_cover_if_needed`) wandern in die
Plan-Konstruktion (`from_rebuild_scope`).
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

**D3 · `refactor(build): flatten build/ module layout`**
Die geteilte Engine (`BuildPlan::run`, `resolve_*`, cover) lebt in den
`build/`-Submodulen; die verschachtelte `build/core.rs`-Ebene entfällt. `build.rs`
enthält die beiden CLI-Einstiege `build`/`rebuild`. Außerdem `build_photo_index`
in ein neutrales Query-Modul ziehen, damit die Lese-Kommandos `place`/`status`
nicht mehr aus `commands::build` importieren.
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

---

## Phase E — BuildPlan als Interface, `rebuild` & Doppelungen auflösen

> Status: Phasen A–D sind umgesetzt (Branch `claude/lib-refactoring-build-…`).
> Phase E baut darauf auf. Das Lib-Interface darf jetzt brechen — Aufrufer in
> `cli/` und `gui/` werden mitgezogen.

**Befund nach A–D:** Die Pipeline (`BuildPlan::run`) ist gut, aber das *Interface*
ist noch aufgebläht: zwei Einstiege (`build` + `rebuild`) und **vier** Typen —
`BuildPlan`, `RebuildScope`, `BuildConfig`, `BuildOptions`. `RebuildScope` ist ein
zweites, fast deckungsgleiches Enum, das nur über `from_rebuild_scope` in
`BuildPlan` übersetzt wird; `BuildOptions` dupliziert zwei Felder aus
`BuildConfig`.

### Zielbild

`BuildPlan` enthält **alle** build-/rebuild-Varianten und *ist* das Interface;
`rebuild`, `RebuildScope` und `BuildOptions` entfallen. Ein einziger Einstieg:

```rust
pub enum BuildPlan {
    Auto { pages: Option<Vec<usize>> },              // build: leeres Layout ⇒ erster Build, sonst inkrementell
    All,                                             // voller Neu-Aufbau (war: rebuild ohne Argument)
    Page(usize),                                     // war: rebuild --page
    Range { start: usize, end: usize, flex: usize }, // war: rebuild --range
    Release { force: bool },                         // finale PDF, kein Neulösen
}

pub struct BuildConfig {   // nur noch Optionen — keine Doppelung mit BuildOptions
    pub skip_pdf: bool,
    pub skip_cache_update: bool,
}

pub fn build(project_root: &Path, plan: BuildPlan, config: &BuildConfig)
    -> Result<CommandOutput<BuildResult>>;
```

Damit verschwindet der `always_commit`-Bool: `Auto` committet via `CommitMode::Auto`,
alle anderen Varianten via `Always` (aus `commit_mode(plan)` ableitbar). Die
Erst-Build-vs-inkrementell-Entscheidung wandert in `resolve_layout` für `Auto`
(Layout leer ⇒ ganzes Buch, sonst veraltete Seiten). `commands.rs` exportiert nur
noch `build`, `BuildPlan`, `BuildConfig`, `BuildResult`, `DpiWarning`.

*Alternative (falls ein einzelner Parameter bevorzugt wird):* `BuildConfig { plan:
BuildPlan, skip_pdf, skip_cache_update }` und `build(project_root, &config)`. Hier
bewusst getrennt gehalten, damit `BuildPlan` der sichtbare Star bleibt.

### Commit-Portionen

**E1 · `refactor(build): unify Full/Incremental into BuildPlan::Auto, add All`**
Interner Schritt, Interface noch stabil. `Full{always_commit}` + `Incremental`
→ `Auto{pages}`; neue Variante `All` für den vollen Neu-Aufbau. `always_commit`-
Feld entfällt; `commit_mode` leitet aus der Variante ab. `resolve_layout::Auto`
verzweigt anhand `mgr.state.layout.is_empty()`. Die `from_*`-Konstruktoren und
`rebuild()` mappen vorerst weiter auf die neuen Varianten (`RebuildScope::All` →
`All` usw.).
*Verify:* erster Build, inkrementell, rebuild-all liefern identische Layouts;
voller Testsatz grün.

**E2 · `refactor(build): make BuildPlan the public build interface`**
*Interface-Bruch.* Neuer Einstieg `build(project_root, plan, &config)`. Entfernen:
`rebuild()`, `RebuildScope`, `from_rebuild_scope`, `from_build_config`. Die
bisherige Konstruktions-/Validierungslogik (Range-/Index-Grenzen, „--pages nicht
mit release") wandert in `build()` bzw. an den Anfang von `run`/`resolve_layout`,
geprüft gegen den geladenen State. Aufrufer im selben Commit umstellen:
`cli/cli/build.rs`, `cli/cli/rebuild.rs` (baut jetzt `BuildPlan::{All,Page,Range}`),
`gui/background/commands/misc.rs` + `page.rs`.
*Verify:* `cli` und `gui` kompilieren gegen das neue Interface; Tests grün.

**E3 · `refactor(build): fold options into BuildConfig, drop BuildOptions`**
`BuildConfig` enthält nur noch `skip_pdf` + `skip_cache_update`; `release`,
`force`, `pages` leben jetzt in `BuildPlan`. `BuildOptions` löschen. Die Regel
`skip_pdf || !preview.write_pdf` einmal in `build()` auswerten. Aufrufer
nachziehen.
*Verify:* keine `BuildOptions`-Referenz mehr; Tests grün.

**E4 · `refactor(build): split plan.rs into plan + resolve`**
Gegen die verbliebene Aufblähung von `plan.rs` (~680 Z.): `enum BuildPlan` + `run`
+ Pipeline-Glue (`commit_mode`/`pdf_target`/`commit_message`) bleiben in `plan.rs`;
die `resolve_*`-Funktionen samt Manual-Page- und Multipage-Hilfen wandern nach
`build/resolve.rs`. Reine Verschiebung.
*Verify:* keine Logikänderung; Tests grün.

### Reihenfolge & Risiko (E)

- E1 ist verhaltensneutral und interface-stabil → zuerst, isoliert testbar.
- E2/E3 brechen das Interface; jeder dieser Commits muss **alle** Aufrufer
  (`cli/`, `gui/`) im selben Schritt mitziehen, sonst bricht der Build.
- Risiko liegt auf der `Auto`-Verzweigung (erster Build vs. inkrementell): die
  bestehenden Tests beider Pfade müssen vor E1 vorhanden und nach E1 grün sein.
