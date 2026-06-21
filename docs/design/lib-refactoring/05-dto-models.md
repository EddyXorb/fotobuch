# Umsetzungsplan 5 — Domänenmodell (`dto_models`)

> Konkretisiert Abschnitt 5 des [Hauptdokuments](./README.md).
> Stand: gegen den aktuellen Code geprüft.

## Worum es geht

`dto_models` heißt „DTO" (logiklose Transporthüllen), trägt aber Domänenlogik,
Persistence und Validierung. Dazu kommen ein paar schwache Typen und Redundanzen.
Die meisten Schritte sind lokal; einige berühren das **YAML-Schema** (serde) und
brauchen Sorgfalt.

## Was bereits erledigt ist

Die Cover-Geometrie (5.2) ist raus aus dem DTO: `spread_width_mm`/`spine_width_mm`
und die tote `CanvasConfig`-Impl leben nicht mehr auf `CoverConfig`, sondern im
Wertobjekt `CoverGeometry` (`dto_models/cover.rs`) — siehe [`02-cover.md`](./02-cover.md).

## Was offen ist (verifiziert)

- **Modulname `dto_models`** suggeriert Logiklosigkeit, ist aber Domänenmodell
  (71 Importstellen crate-weit).
- **`ProjectState` als Gott-Objekt-Keim** (`state.rs`): Persistence (`load:23`,
  `save:34`), Domain-Queries (`page_dimensions_mm:64`, `auto_page_indices:51`,
  `has_cover:45`) und Validierung (`check_validity:91`) in einem Typ.
- **`BookLayoutSolverConfig::validate(total_photos)`** (`:184`) braucht
  Anwendungszustand (Fotoanzahl) auf einem Config-DTO.
- **`CanvasConfig`-Trait** in `config/book_config.rs:31` definiert — wird aber von
  `BookConfig` *und* `CoverGeometry` (beide dto_models) implementiert und nur vom
  Solver konsumiert. (Der frühere Vorschlag „näher an den Solver" ist damit
  überholt: das Trait gehört in dto_models, nur nicht in eine spezifische
  Config-Datei.)
- **`aspect_ratio()` 3× identisch** (`photos/photo_file.rs:31`,
  `solver/data_models/layout.rs:73`, `…/canvas.rs:42`).
- **`PhotoGroup.sort_key: String`** (`photos/photo_group.rs:11`) ist faktisch immer
  ein ISO-8601-Timestamp.
- **`CoverMode`-Default-Falle:** `#[default]` = `Free` (`cover_config.rs:13`), aber
  `CoverConfig::default().mode` = `Split` (`:128`).
- **Deprecated-Felder** im aktiven DTO (`book_layout_solver_config.rs:59` ff.,
  `mip_rel_gap` u. a.).
- **`BookConfig` enthält `cover`/`appendix` als Pflicht-Kinder**
  (`book_config.rs:25,28`) → tiefe Zugriffspfade.
- **`LayoutPage.page`** (`layout/layout_page.rs:27`) ist eine denormalisierte
  Index-Invariante (`== layout[i]`).

## Commit-Portionen

### Kleine, lokale Schritte

**I1 · `fix(models): unify the CoverMode default`**
`#[default]` (`Free`) und `CoverConfig::default().mode` (`Split`) widersprechen
sich — je nach Zugriff (`CoverMode::default()` bzw. ein per `#[serde(default)]`
fehlendes `mode`-Feld) entsteht ein anderer Default. Auf *einen* Wert festlegen
und mit einem Test `CoverConfig::default().mode == CoverMode::default()` verriegeln.
*Verify:* Test + bestehende Cover-Tests grün.

**I2 · `refactor(models): deduplicate aspect_ratio`**
Eine Quelle für `width / height` (freier Helfer `aspect_ratio(w, h)` oder ein
Trait); die drei Stellen delegieren.
*Verify:* Tests grün.

**I3 · `refactor(models): extract CanvasConfig into its own module`**
Trait aus `config/book_config.rs` in `dto_models/canvas_config.rs` ziehen
(implementiert von `BookConfig`/`CoverGeometry`, konsumiert vom Solver). Reine
Verschiebung.
*Verify:* Tests grün.

### Logik von den DTOs lösen

**I4 · `refactor(models): move BookLayoutSolverConfig::validate out of the DTO`**
Die Validierung braucht `total_photos` (Anwendungszustand) → Validator-Modul statt Methode auf dem Config-DTO; der DTO bleibt reine Daten.
*Verify:* gleiche Fehlerfälle; Tests grün.

**I5 · `refactor(models): separate ProjectState persistence from the domain type`**
`load`/`save` (Datei-I/O) aus `ProjectState` in ein Repository-/IO-Modul (z.B. beim `state_manager`). `ProjectState` behält die
Domain-Queries; `check_validity` ggf. in einen Validator. Ziel: ein Typ, eine
Verantwortung.
*Verify:* Laden/Speichern unverändert (Round-Trip-Test); Tests grün.

### Schema-berührende Schritte (serde-Vorsicht)

**I6 · `refactor(models): drop deprecated BookLayoutSolverConfig fields`**
`mip_rel_gap` u. a. entfernen. Serde ignoriert unbekannte Felder bestehender YAML
beim Laden, daher kein Migrationsschritt nötig (Round-Trip-Test als Absicherung).
*Verify:* alte YAML lädt ohne Fehler; Tests grün.

### LayoutPage.page` entfernen

**I7 `LayoutPage.page` entfernen.** Denormalisierter Index; das Array ordnet bereits
  kanonisch. Aber: `page` wird serialisiert und als Identität in Diffs genutzt —
  Wegfall berührt `renumber_pages`, `check_validity`, `state_diff` und das
  YAML-Schema.

### Strukturell (hohe Streuung)

**I8 · `refactor: rename module dto_models → models`**
Crate-weite, mechanische Umbenennung (71 Importstellen, inkl. `cli`/`gui`) plus
Re-Export-Pfade. Rein kosmetisch, hoher Diff — bewusst als letzter Schritt, damit
die inhaltlichen Commits klein bleiben.
*Verify:* alles kompiliert; Tests grün.
