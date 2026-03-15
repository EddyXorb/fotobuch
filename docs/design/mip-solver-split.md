# MIP-Solver Split für große Instanzen

## Problem

Bei mehr als ~100 Bildern wird das MIP schwer lösbar. Lösung: Problem in k Teilprobleme zerlegen, sequenziell lösen, Ergebnisse zusammenführen.

---

## Neue Config-Felder in `BookLayoutSolverConfig`

```rust
pub max_photos_for_split: usize,       // default: 100 — Trigger-Schwelle
pub split_group_boundary_slack: usize, // default: 5   — erlaubte Abweichung vom Idealteilpunkt
```

Trigger: `n_photos > max_photos_for_split` → aufteilen, sonst bisheriger Ablauf.

---

## Teilung (`split.rs` oder inline in `book_layout_solver.rs`)

### Anzahl Teilprobleme

```
k = ceil(n / max_photos_for_split)
```

### Splitpunkte bestimmen

Für jeden Idealteilpunkt `target_i = round(i * n / k)` mit `i = 1..k-1`:

1. Prüfe alle Gruppengrenzen im Fenster `[target_i - slack, target_i + slack]`
2. Wähle die Gruppengrenze, die `target_i` am nächsten liegt — falls vorhanden
3. Sonst: Split exakt bei `target_i`

Ergebnis: `k-1` Splitpunkte → k Foto-Ranges `[s_0..s_1), [s_1..s_2), ..., [s_{k-1}..n)`

### Teilproblem-Parameter

Alle Parameter bleiben identisch außer:

| Parameter      | Subproblem i                                              |
|----------------|-----------------------------------------------------------|
| `page_target`  | `round(page_target * photos_i / n)`, letztes: Rest       |
| `page_max`     | `round(page_max * photos_i / n)`, letztes: Rest          |
| `page_min`     | 1 (global-min gilt nicht pro Teilproblem)                 |
| `search_timeout` | `search_timeout / k`                                  |

**Invariante:** `sum(page_target_i) == page_target`, `sum(page_max_i) == page_max`
Sichergestellt durch: letzte Partition erhält den Rest statt erneutes Runden.

Mindestsicherheit: `page_target_i >= 1`, `page_max_i >= page_target_i` — klemme wenn nötig.

---

## Fallback

Falls `solve_mip` für ein Teilproblem fehlschlägt (`Infeasible` oder `Timeout`): der Solver-Hint wird direkt als Ergebnis verwendet.

Der Hint-`PageAssignment` wird vor dem MIP-Aufruf aus der greedy Split-Logik generiert (Splitpunkte an Gruppengrenzen) und bereits an `solve_mip(..., Some(&hint))` übergeben. Bei Fehler: `hint` direkt weiter an Local Search übergeben.

Kein separates Fallback-Modul nötig — der Hint dient als Warm-Start *und* als Fallback.

---

## Ergebnis-Zusammenführung

Jedes Teilproblem liefert ein `PageAssignment` mit Layout-Cache.
Konkatenation:
- Alle `PageAssignment::cuts` hintereinander (jeweils Offset aufaddieren)
- Alle Caches mergen (Schlüssel sind Foto-Ranges, keine Konflikte)
- `BookLayout::pages` = verkettete Seitenlayouts beider Phasen (MIP + Local Search)

---

## Ablauf in `solve_book_layout`

```
if n <= max_photos_for_split:
    → bisheriger Ablauf (unverändert)
else:
    1. split_photos(photos, groups, params) → Vec<SubRange>
    2. for each sub_range:
       a. sub_params = derive_sub_params(params, sub_range, n, k)
       b. sub_groups = GroupInfo::from_photos(&photos[sub_range])
       c. hint = greedy_assignment(sub_groups, sub_params)
       d. assignment = solve_mip(sub_groups, sub_params, Some(&hint))
                       .unwrap_or(hint)
       e. if enable_local_search: improve(assignment, ...)
       f. collect sub-result
    3. merge all sub-results → BookLayout
```

---

## Neue Dateien / Änderungen

| Datei | Änderung |
|-------|----------|
| `dto_models/config/book_layout_solver_config.rs` | 2 neue Felder + Defaults |
| `solver/book_layout_solver.rs` | Split-Logik + Merge, neuer Pfad in `solve_book_layout` |
| `solver/book_layout_solver/split.rs` (neu) | `split_photos()`, `derive_sub_params()`, `greedy_assignment()` |

---

## Tests

- **`split.rs` unit**: Splitpunkte landen an Gruppengrenzen wenn innerhalb slack; ohne Gruppengrenze exakt bei Idealteilpunkt; Invariante `sum(page_target_i) == page_target`
- **`split.rs` unit `greedy_assignment`**: Ergebnis ist immer feasible; Seitengrößen in `[p_min, p_max]`
- **Integration**: 150 Fotos → Split findet statt, Gesamtseitenzahl ≈ `page_target`, alle Fotos im Ergebnis
