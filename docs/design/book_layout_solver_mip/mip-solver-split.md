# MIP-Solver Split für große Instanzen

## Problem

Bei mehr als ~100 Bildern wird das MIP schwer lösbar. Das Problem wird in k Teilprobleme zerlegt, sequenziell gelöst und zusammengeführt.

## Konfiguration

- `max_photos_for_split` (Default: 100) — Trigger-Schwelle
- `split_group_boundary_slack` (Default: 5) — erlaubte Abweichung vom Idealteilpunkt

## Architektur

`PageAssignmentSolver` (in `page_assignment_solver.rs`) ist der einzige Einstiegspunkt für Seitenzuteilung. Kapselt Hint-Berechnung, Splitting, MIP-Aufruf und Zusammenführung.

## Ablauf

```
k = ceil(n / max_photos_for_split)   // k=1 → kein Split

Für jeden Split-Bereich:
  sub_params = abgeleitete Parameter (anteilige page_target/page_max)
  hint       = create_start_solution(sub_params, sub_photos)
  assignment = solve_mip(sub_groups, sub_params, hint)
               .unwrap_or(hint)   // Fallback bei MipError

merge(assignments) → globales PageAssignment
```

## Splitpunkte

Für jeden Idealteilpunkt `target_i = round(i * n / k)`:
1. Suche Gruppengrenze im Fenster `[target_i - slack, target_i + slack]`
2. Wähle die nächstgelegene — falls vorhanden
3. Sonst: Split exakt bei `target_i`

## Teilproblem-Parameter

Alle Parameter identisch außer:

| Parameter        | Subproblem i                                       |
| ---------------- | -------------------------------------------------- |
| `page_target`    | `page_target / k` + 1 wenn `i < (page_target % k)` |
| `page_max`       | `page_max / k` + 1 wenn `i < (page_max % k)`       |
| `page_min`       | 1                                                  |
| `search_timeout` | `search_timeout / k`                               |

Invariante: `sum(page_target_i) == page_target`, `sum(page_max_i) == page_max`.
