# Umsetzungsplan 2 — Cover-Logik am richtigen Ort

> Konkretisiert Abschnitt 2 (und 5.2) des [Hauptdokuments](./README.md).
> Stand: nach dem umgesetzten build-Refactoring.

## Status: was bereits erledigt ist

Die in Abschnitt 2 beschriebene **Konsolidierung der verstreuten Cover-Logik ist
abgeschlossen.** Sie liegt jetzt gebündelt in
`src/commands/build/build_layout/cover_page.rs`:

- `build_cover_page` — neue Cover-Seite (structured).
- `update_cover_page` — verzweigt sauber in `update_cover_free` /
  `update_cover_structured`.
- `split_cover_photos` — Cover-Fotos abspalten.
- `CoverCanvasConfig` — Wrapper, der dem Solver die volle Spread-Breite liefert.

Die früher dreifach gestreute Fallunterscheidung *free vs. structured* ist damit
an einer Stelle. **Hier ist kein weiteres Refactoring nötig.**

## Was noch offen ist: Cover-Geometrie am falschen Ort

Die **Geometrie-Berechnungen leben weiterhin als Methoden auf dem Daten-DTO**
`CoverConfig` (`src/dto_models/config/cover_config.rs`):

- `spread_width_mm(inner_page_count)` — Gesamtbreite (front+back+spine).
- `spine_width_mm(inner_page_count)` — Rückenbreite aus Seitenzahl.
- `resolved_spine_text(book_title)` — Text-Fallback.

Das sind keine Getter, sondern Rechnungen, die zudem einen *Laufzeitwert*
(`inner_page_count`) als Parameter brauchen — ein DTO sollte das nicht. Konkrete
Folgeprobleme:

1. **`inner_page_count` wird durchgereicht** an jeden Aufrufer:
   `state.rs:68` (`page_dimensions_mm`), `commands/page/info.rs:18`,
   `solver/cover_solver.rs:119`, `cover_page.rs:138`.
2. **Tote, irreführende zweite `CanvasConfig`-Impl:** `impl CanvasConfig for
   CoverConfig` (`cover_config.rs:176`) hat **keinen Consumer** — der echte
   Cover-Canvas ist `CoverCanvasConfig` in `cover_page.rs`. Die DTO-Impl liefert
   sogar bewusst die *falsche* Breite (nur `front_back_width_mm`, mit Warnhinweis
   im Doc-Comment). Klassische Falle.
3. **`resolved_spine_text` ist repo-weit ohne Consumer** — vermutlich Altlast.

## Entscheidung

Ein **kleines, fokussiertes Refactoring** lohnt sich (kein „nichts zu tun"):
die Geometrie als eigenständiges Wert-Objekt aus dem DTO herauslösen.

## Zielbild: `CoverGeometry`-Wertobjekt

Ein Wertobjekt bündelt `CoverConfig` + `inner_page_count` und besitzt die
Rechnungen; `CoverConfig` wird wieder ein reines Daten-DTO.

```rust
// dto_models/cover.rs (Domänen-Schicht, da von state/info/solver/build genutzt)
pub struct CoverGeometry<'a> {
    cover: &'a CoverConfig,
    inner_page_count: usize,
}

impl<'a> CoverGeometry<'a> {
    pub fn new(cover: &'a CoverConfig, inner_page_count: usize) -> Self { /* … */ }
    pub fn spread_width_mm(&self) -> f64 { /* von CoverConfig hierher */ }
    pub fn spine_width_mm(&self) -> f64 { /* von CoverConfig hierher */ }
    pub fn height_mm(&self) -> f64 { self.cover.height_mm }
}

impl CanvasConfig for CoverGeometry<'_> { /* die EINE, korrekte Spread-Variante */ }
```

Vorteile: `inner_page_count` wird einmal gebunden statt überall durchgereicht;
es gibt nur noch *eine* korrekte `CanvasConfig`-Impl; `CoverCanvasConfig`
(cover_page.rs) und die tote DTO-Impl entfallen beide.

## Commit-Portionen

**F1 · `feat(cover): add CoverGeometry value object`**
Neues `dto_models/cover.rs` mit `CoverGeometry` (Geometrie + `CanvasConfig`-Impl).
Die bisherigen `CoverConfig::spread_width_mm`/`spine_width_mm` bleiben vorerst und
delegieren an `CoverGeometry`, damit nichts bricht.
*Verify:* neue Unit-Tests für die Geometrie; voller Testsatz grün.

**F2 · `refactor(cover): route consumers through CoverGeometry`**
`state.rs:68`, `commands/page/info.rs:18`, `solver/cover_solver.rs:119` und
`cover_page.rs` bauen ein `CoverGeometry` statt `cover.spread_width_mm(n)` zu
rufen. Der lokale `CoverCanvasConfig` wird durch `CoverGeometry` ersetzt (gleiche
Form) und gelöscht.
*Verify:* identische Maße wie vorher (reine Umverdrahtung); Tests grün.

**F3 · `refactor(cover): make CoverConfig a pure data DTO`**
Aus `CoverConfig` entfernen: `spread_width_mm`, `spine_width_mm` (jetzt in
`CoverGeometry`) und die **tote** `impl CanvasConfig for CoverConfig`. Für
`resolved_spine_text` zuerst prüfen, ob wirklich ungenutzt — wenn ja, entfernen;
sonst nach `CoverGeometry` ziehen.
*Verify:* `cargo build` ohne `dead_code`-/unused-Warnungen; Tests grün.

## Risiko

- Gering: F2 ist eine mechanische Umverdrahtung mit unveränderter Formel — die
  berechneten Maße müssen vor/nach identisch sein (durch bestehende Tests in
  `cover_config.rs` und `cover_solver.rs` abgedeckt; ggf. einen Vergleichstest
  ergänzen).
- Einzig zu klären: endgültiger Modul-Ort. `dto_models/cover.rs` ist gewählt, weil
  die Consumer (`state`, `page/info`, `solver`, `build`) schichtenübergreifend
  sind — eine Heimat im build-Modul (wie die schon erledigte Konsolidierung) wäre
  hier zu eng.
