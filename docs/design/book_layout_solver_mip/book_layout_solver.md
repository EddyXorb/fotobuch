# Book Layout Solver

Verteilt eine sortierte Bildsequenz (gruppiert, chronologisch) auf Buchseiten. Zwei Phasen:

1. **MIP-Solver** (`mip/`): Findet eine global optimale Seitenzuteilung unter strukturellen Constraints.
2. **Local Search** (`local_search/`): Verbessert die Zuteilung iterativ basierend auf tatsächlicher Layout-Qualität.

Ein **Layout-Cache** verbindet beide Phasen und verhindert redundante GA-Aufrufe.

## Datenmodell

### `PageAssignment`

Partitionierung der Bildsequenz als Schnittpunkte: Seite j enthält Bilder `[cuts[j-1]..cuts[j])`. `cuts[0]=0`, `cuts[last]=n`.

### `Params`

| Feld                      | Bedeutung                                           |
| ------------------------- | --------------------------------------------------- |
| `page_target`             | Ziel-Seitenanzahl                                   |
| `page_min/max`            | Erlaubter Bereich                                   |
| `photos_per_page_min/max` | Bilder pro Seite                                    |
| `group_max_per_page`      | Max. Gruppen pro Seite                              |
| `group_min_photos`        | Mindestanteil einer Gruppe wenn sie gesplittet wird |

### `PageCost` / `AssignmentCost`

`PageCost` entspricht dem `CostBreakdown` des Page-Layout-Solvers. `AssignmentCost` aggregiert über alle Seiten; primäres Kriterium ist `coverage` (Weißraum), sekundär `average`. Implementiert `Ord`.

### `GroupInfo`

Kumulative Gruppengrößen für schnelle Zugriffe (`group_of_photo`, `group_range`).

## Constraint-Prüfung (`feasibility.rs`)

Prüft `PageAssignment` gegen alle Hard-Constraints in dieser Reihenfolge (early return):

1. Seitenanzahl in `[b_min, b_max]`
2. Jede Seitengröße in `[p_min, p_max]`
3. Max. Gruppen pro Seite ≤ `g_max`
4. `g_min`-Regel: Teil-Gruppe auf Seite j muss ≥ `g_min` Bilder haben (wenn Gesamtgruppe ≥ `g_min`)

Wird nach MIP-Solve und nach jeder Perturbation in der Local Search aufgerufen.

## MIP-Solver (`mip/`)

Verwendet `good_lp` mit HiGHs-Backend. Formulierung in `docs/design/book_layout_solver_mip/` (Typst-Dokument `page_assignment_mip.typ`). Extraktion der Schnittpunkte aus kumulativen `g_{l,j}`-Variablen. Inaktive Seiten werden beim Aufbau von `PageAssignment` übersprungen.

## Layout-Cache (`cache.rs`)

Schlüssel: `Range<usize>` (Bildbereich). Speichert nur wenn besser (niedrigerer Coverage-Cost) als vorhandener Eintrag (`insert_if_better`). Alle Local-Search-Aufrufe gehen über den Cache.

## Schnittstelle zum Page-Layout-Solver

```rust
trait PageLayoutEvaluator {
    fn evaluate(&mut self, photos: &[Photo]) -> PageCost;
}
```

`GaPageLayoutEvaluator` implementiert das Trait und delegiert an `page_layout_solver::run_ga`.

## Local Search

Gerichtete Nachbarschaftssuche: Schnittpunkte an Seiten mit `coverage > max_coverage_cost` verschieben (delta ±1, ±2, …). Worst-first-Selektion. Abbruch bei Timeout oder wenn alle Seiten gut genug.

## Öffentliche API

```rust
pub fn solve(
    photos: &[Photo],
    groups: &GroupInfo,
    params: &Params,
    evaluator: &mut impl PageLayoutEvaluator,
) -> Result<SolverResult, SolverError>
```

Ablauf: `params.validate()` → MIP (mit Hint/Fallback) → Local Search → `SolverResult`.

Bei `MipError`: Fallback auf `create_start_solution` (greedy Zuteilung).
