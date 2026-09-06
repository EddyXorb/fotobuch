# Umsetzungsplan 3 — Datei- & Modulstruktur des build-Pfads

> Konkretisiert Abschnitt 3 des [Hauptdokuments](./README.md).
> Stand: G1–G5 umgesetzt — dieser Plan ist vollständig abgeschlossen.

## Status: G1–G5 umgesetzt

Alle fünf Commit-Portionen aus diesem Plan sind umgesetzt:

- **G1** — `helpers.rs` aufgelöst in `build/cache.rs` (Cache) und
  `build/render.rs` (PDF); `CommitMode` liegt jetzt in `plan.rs`.
- **G2** — Modul `build_layout` → `layout` (Pfad `commands::build::layout`);
  Methode `BuildPlan::build_layout()` → `resolve_layout()`.
- **G3** — `collect_photos_as_groups` liegt jetzt in `models/photos.rs` neben
  `build_photo_index`; deutsche Kommentare übersetzt.
- **G4** — Engine `build_cover_page` → `solve_cover_page` (einheitliches
  `solve_*`); Layout-Schicht einheitlich auf `*_layout` umbenannt:
  `full_book_layout`/`outdated_pages_layout`/`page_layout`/`page_range_layout`.
- **G5** — Doc-Comment in `build.rs:1` deckt jetzt auch die rebuild-Varianten ab.

Aktuelle Struktur:

```
commands/build.rs                Einstieg: build(), BuildConfig, BuildResult, DpiWarning
commands/build/plan.rs            BuildPlan + run-Pipeline + resolve_layout() + commit/pdf/cache-Glue
commands/build/cache.rs           CacheRefresh, refresh_preview_cache, refresh_final_cache, log_dpi_warnings
commands/build/render.rs          RenderContext, PdfTarget, render_pdf
commands/build/errors.rs          BuildError
commands/build/layout.rs          full_book_layout / outdated_pages_layout / page_layout / page_range_layout
commands/build/layout/{multi_page,single_page,cover_page}.rs   solve_*-Engines (update_cover_page bleibt Update)
models/photos.rs                  build_photo_index, collect_photos_as_groups
```

## Status: was bereits erledigt ist (Ausgangslage)

Alle ursprünglichen Befunde aus Abschnitt 3 sind **umgesetzt**:

- `build/core/` und der Re-Export-Knoten `build/core.rs` existieren nicht mehr;
  der Pfad ist verflacht.
- `rebuild.rs` ist verschwunden — `rebuild` ist als `BuildPlan`-Variante in den
  einen `build()`-Einstieg aufgegangen (kein Schwester-Kommando mehr).
- `build_photo_index` lebt jetzt in `dto_models/photos.rs`; `place`/`status`
  importieren es aus `dto_models`, nicht mehr aus `commands::build`.
- Die driftende Doc-Numerierung (altes `rebuild.rs`) ist mit der Datei weg.

Aktuelle Struktur:

```
commands/build.rs                     Einstieg: build(), BuildConfig, BuildResult, DpiWarning
commands/build/plan.rs                BuildPlan + run-Pipeline + commit/pdf/cache-Glue
commands/build/helpers.rs             CacheRefresh, RenderContext, render_pdf, refresh_*, CommitMode, collect_photos_as_groups
commands/build/build_layout.rs        build_full_book / _outdated_pages / _page / _page_range
commands/build/build_layout/{multi_page,single_page,cover_page}.rs   Solver-Engines
```

## Ausgangsbefund (historisch): Benennung & Kohäsion

Die Aufteilung ist gut, aber drei Namens-/Kohäsionsprobleme bleiben.

1. **`helpers.rs` ist ein Sammelname** (185 Z.) mit **vier** unverwandten Themen:
   - Cache: `CacheRefresh`, `refresh_preview_cache`, `refresh_final_cache`,
     `log_dpi_warnings`
   - PDF: `RenderContext`, `PdfTarget`, `render_pdf`
   - Commit: `CommitMode`
   - Foto-Query: `collect_photos_as_groups`

   „helpers" sagt nichts; die Themen gehören getrennt.

2. **Namenskollision `build_layout`.** Es gibt gleichzeitig
   - das **Modul** `commands/build/build_layout.rs` und
   - die **Methode** `BuildPlan::build_layout()` (`plan.rs:35`),

   die *unterschiedliche* Dinge sind (die Methode ruft das Modul auf). Zusätzlich
   ist der Pfad `commands::build::build_layout` redundant („build_build_layout").

3. **Uneinheitliche Verben in der Layout-Schicht.** Die Schicht-Funktionen heißen
   `build_full_book` / `build_outdated_pages` / `build_page` / `build_page_range`
   (Präfix `build_`), die Engines darunter `solve_multipage` / `solve_single_page`
   (Verb `solve`) vs. `build_cover_page` / `update_cover_page` (Verb `build`/
   `update`). Drei Verben für „Layout erzeugen".

Nicht hier behandelt: die CLI-Fehlerstrings in `build_layout.rs` (z. B. `:57,69`)
und `plan.rs:120` — die gehören zu Abschnitt 10 / dessen Plan.

## Entscheidung

Ein **kleines, rein strukturelles Refactoring** (Verschieben + Umbenennen, keine
Logikänderung) lohnt sich.

## Commit-Portionen

**G1 · `refactor(build): split helpers.rs into cache + render modules`**
`helpers.rs` auflösen in:
- `build/cache.rs` — `CacheRefresh`, `refresh_preview_cache`, `refresh_final_cache`,
  `log_dpi_warnings`.
- `build/render.rs` — `RenderContext`, `PdfTarget`, `render_pdf`.
- `CommitMode` zu `plan.rs` (wird nur dort gebraucht) oder in ein winziges
  `build/commit.rs`.
Reine Verschiebung + Import-Anpassung.
*Verify:* keine Logikänderung; Tests grün.

**G2 · `refactor(build): rename build_layout → layout, method → resolve_layout`**
Modul `build_layout.rs`/`build_layout/` → `layout.rs`/`layout/` (Pfad
`commands::build::layout`). Methode `BuildPlan::build_layout()` → `resolve_layout()`
(beschreibt „Layout berechnen", kollidiert nicht mehr mit dem Modulnamen).
*Verify:* Tests grün.

**G3 · `refactor(build): rehome collect_photos_as_groups as a neutral photo query`**
`collect_photos_as_groups` ist eine reine Query über `state.layout`/`state.photos`
(keine build-Logik) — nach `dto_models/photos.rs` neben `build_photo_index` ziehen
(konsistent zu dessen bereits erfolgtem Umzug). Deutsche Kommentare („Sammelt alle
Fotos…", „Nach Originalgruppe aufteilen") ins Englische.
*Verify:* `build` hängt für diese Query nicht mehr an einem build-internen Helfer;
Tests grün. (Falls der Einzel-Consumer gegen den Umzug spricht: alternativ in
`build/layout.rs` belassen, aber Kommentare trotzdem übersetzen.)

**G4 · `refactor(build): unify layout-engine verbs`**
Ein Verb je Rolle: Engines einheitlich `solve_*` (also `solve_cover_page` statt
`build_cover_page`; `update_cover_page` bleibt, da es bewusst *aktualisiert*).
Schicht-Funktionen einheitlich auf ein Schema bringen (z. B. `*_layout`:
`full_book_layout`/`page_layout`/…), sodass „solve = Engine, *_layout = Schicht"
ablesbar ist. Reine Umbenennung.
*Verify:* Tests grün.

**G5 · `docs(build): fix stale module doc`**
`build.rs:1` sagt `//! \`fotobuch build\` command`, das Modul bedient aber auch
alle rebuild-Varianten — Doc-Comment entsprechend fassen.

## Risiko

Sehr gering: G1–G3 sind mechanische Verschiebungen/Umbenennungen ohne
Verhaltensänderung, durch die bestehenden Build-Tests abgedeckt. G4/G5 sind reine
Namens-/Doc-Änderungen.
