# MIP-Solver Split für große Instanzen

## Problem

Bei mehr als ~100 Bildern wird das MIP schwer lösbar. Lösung: Problem in k Teilprobleme zerlegen, sequenziell lösen, Ergebnisse zusammenführen.

---

## Neue Config-Felder in `BookLayoutSolverConfig`

```rust
pub max_photos_for_split: usize,       // default: 100 — Trigger-Schwelle
pub split_group_boundary_slack: usize, // default: 5   — erlaubte Abweichung vom Idealteilpunkt
```

---

## Architektur: `PageAssignmentSolver`

Neuer Wrapper `page_assignment_solver.rs` (Submodul von `book_layout_solver`) — einziger Einstiegspunkt für Seitenzuteilung. Kapselt: Hint-Berechnung, Splitting, MIP-Aufruf, Zusammenführung.

`book_layout_solver.rs` ruft ihn in einer Zeile auf:

```rust
let assignment = PageAssignmentSolver::new(params).solve(groups)?;
```

Intern kennt `PageAssignmentSolver` den MIP-Solver und entscheidet selbst ob gesplittet wird.

---

## Ablauf in `PageAssignmentSolver::solve`

```
k = ceil(n / max_photos_for_split)   // k=1 → kein Split

split_points = compute_split_points(groups, params, k)  // k-1 Punkte
sub_ranges   = split_points → k Foto-Ranges

for each sub_range:
    sub_groups = groups.slice(sub_range)
    sub_params = derive_sub_params(params, sub_range.len(), n, k, remaining)
    hint       = greedy_assignment(sub_groups, sub_params)
    assignment = solve_mip(sub_groups, sub_params, Some(&hint))
                 .unwrap_or(hint)
    collect assignment

merge(assignments) → PageAssignment
```

---

## Splitpunkte bestimmen

Für jeden Idealteilpunkt `target_i = round(i * n / k)` mit `i = 1..k-1`:

1. Suche Gruppengrenze im Fenster `[target_i - slack, target_i + slack]`
2. Wähle die nächstgelegene — falls vorhanden
3. Sonst: Split exakt bei `target_i`

---

## Teilproblem-Parameter

Alle Parameter bleiben identisch außer:

| Parameter        | Subproblem i                                        |
|------------------|-----------------------------------------------------|
| `page_target`    | `round(page_target * photos_i / n)`, letztes: Rest |
| `page_max`       | `round(page_max * photos_i / n)`, letztes: Rest    |
| `page_min`       | 1                                                   |
| `search_timeout` | `search_timeout / k`                               |

**Invariante:** `sum(page_target_i) == page_target`, `sum(page_max_i) == page_max` — letzte Partition erhält den Rest.

Mindestsicherheit: `page_target_i >= 1`, `page_max_i >= page_target_i`.

---

## Hint / Fallback

`greedy_assignment` teilt Fotos des Teilproblems gleichmäßig auf `page_target_i` Seiten auf, bevorzugt Schnitte an Gruppengrenzen. Dient als:
- **Warm-Start** für den MIP-Solver
- **Fallback** bei `MipError` (`.unwrap_or(hint)`)

---

## Zusammenführung

- `PageAssignment::cuts` aller Teilprobleme hintereinander (Offset aufaddieren)
- Ergibt ein einzelnes globales `PageAssignment` das `book_layout_solver.rs` weiter an Local Search übergibt

---

## Neue Dateien / Änderungen

| Datei | Änderung |
|-------|----------|
| `dto_models/config/book_layout_solver_config.rs` | 2 neue Felder + Defaults |
| `solver/book_layout_solver/page_assignment_solver.rs` (neu) | `PageAssignmentSolver`, `compute_split_points()`, `derive_sub_params()`, `greedy_assignment()`, `merge()` |
| `solver/book_layout_solver.rs` | Aufruf auf eine Zeile reduziert |

---

## Tests

- **`compute_split_points`**: Snap an Gruppengrenze wenn innerhalb slack; exakt bei Idealteilpunkt sonst; Invariante `sum(page_target_i) == page_target`
- **`greedy_assignment`**: Ergebnis immer feasible; Seitengrößen in `[p_min, p_max]`
- **Integration**: 150 Fotos → Split findet statt, Gesamtseitenzahl ≈ `page_target`, alle Fotos im Ergebnis
