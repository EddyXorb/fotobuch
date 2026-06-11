# DP-Solver: Implementierungsplan

Ersetzt den MIP-Solver für die Seitenzuordnung (Phase 1 in `solve_book_layout`)
durch die exakte DP aus `dp.typ`. Ziel: identisches Optimierungsergebnis (gleiche
Zielfunktion), Laufzeit < 1 s für 1000 Bilder / 30 Gruppen, deterministisch.

## Modulstruktur

Neue Datei `src/solver/book_layout_solver/dp.rs`, deklariert als `mod dp;` in
`src/solver/book_layout_solver.rs` (kein `mod.rs`, gemäß Projektkonvention).

Öffentliche API des Moduls:

```rust
pub enum DpError { Infeasible }   // thiserror, analog zu MipError

pub fn solve_dp(groups: &GroupInfo, params: &Params) -> Result<PageAssignment, DpError>
```

`Params` = `crate::dto_models::BookLayoutSolverConfig` (Re-Export in `model.rs`).
Genutzte Felder: `page_target` (= s), `page_min`/`page_max` (= b_min/b_max),
`photos_per_page_min`/`photos_per_page_max` (= p_min/p_max), `group_min_photos`
(= g_min), `group_max_per_page` (= g_max), `weight_even`/`weight_split`/`weight_pages`
(= w1/w2/w3). **Nicht** genutzt: `search_timeout` (bleibt für Local Search),
`mip_rel_gap`, `max_photos_for_split`, `split_group_boundary_slack` (werden entfernt).

## Algorithmus (siehe dp.typ für Herleitung)

Alle Indizes 0-basiert. `n = groups.total_photos()`. Bei `n == 0`: wird upstream
abgefangen (`solve_book_layout` returnt früher), `solve_dp` darf trotzdem
`Err(Infeasible)` liefern.

### 1. Vorberechnung — O(n)

```rust
// photo_group[i] = Gruppenindex von Bild i (γ aus dp.typ)
let photo_group: Vec<usize>;        // via groups.group_range(l) befüllen, nicht
                                    // n-mal groups.group_of_photo() aufrufen (O(n·k))
// group_start[l], group_end[l]     // aus groups.group_range(l)
let n_bar = n as f64 / params.page_target as f64;
```

### 2. Intervall-Prüfung und -Kosten — je O(1)

```rust
// F(a, b) aus dp.typ: Zulässigkeit der Seite [a, b)
fn interval_feasible(a: usize, b: usize, ...) -> bool
```

Prüfungen (Reihenfolge wie hier, early return):

1. Seitengröße ist durch die Schleifengrenzen bereits in `[p_min, p_max]`.
2. `photo_group[b-1] - photo_group[a] + 1 <= group_max_per_page`.
3. Spaltungsregel nur für Randgruppen `l ∈ {photo_group[a], photo_group[b-1]}`
   (bei Gleichheit nur einmal prüfen): Anteil
   `t = min(b, group_end[l]) - max(a, group_start[l])`; falls `t < group_size(l)`,
   verlange `group_size(l) >= group_min_photos && t >= group_min_photos`.

Kosten:

```rust
// c_page(a,b) = w1 * |(b-a) - n_bar|
// κ(c)        = w2, falls c > 0 && photo_group[c-1] == photo_group[c], sonst 0.0
```

### 3. DP-Tabellen

```rust
// Flache Vec, Index idx(m, i) = m * (n + 1) + i
let mut dp:     Vec<f64> = vec![f64::INFINITY; (b_max + 1) * (n + 1)];
let mut parent: Vec<u32> = vec![u32::MAX; (b_max + 1) * (n + 1)]; // gewählte Seitengröße p
dp[idx(0, 0)] = 0.0;
```

Schleifen (Pruning über erreichbare Zustände):

```text
for m in 1..=page_max:
  for i in max(1, m * p_min) ..= min(n, m * p_max):
    for p in p_min ..= min(p_max, i):
      a = i - p
      if dp[idx(m-1, a)].is_finite() && interval_feasible(a, i):
        cand = dp[idx(m-1, a)] + kappa(a) + page_cost(a, i)
        bei Verbesserung: dp[idx(m, i)] = cand; parent[idx(m, i)] = p
```

Tie-Breaking deterministisch: strikt `cand < dp[...]` (erste = kleinste Seitengröße
gewinnt bei Gleichstand).

### 4. Ergebnisauswahl und Backtracking

```text
best_m = argmin über m in [page_min, page_max] von dp[idx(m, n)] + weight_pages * |m - page_target|
         (strikt <, d. h. kleinstes m gewinnt bei Gleichstand)
falls kein endlicher Wert: Err(DpError::Infeasible)

cuts: von (n, best_m) rückwärts via parent p: i -> i - p, m -> m - 1, bis (0, 0);
cuts umdrehen, beginnt mit 0, endet mit n  ->  PageAssignment::new(cuts)
```

## Integration

### `src/solver/book_layout_solver.rs`

- `mod dp;` hinzufügen, `mod mip;` entfernen.
- `SolverError::MipFailed(mip::MipError)` ersetzen durch
  `SolverError::AssignmentFailed(dp::DpError)` (oder gleichnamig); Doku-Kommentare
  („MIP Phase") auf DP umformulieren.

### `src/solver/book_layout_solver/page_assignment_solver.rs`

Die Splitting-Maschinerie (`compute_split_points`, `derive_sub_params`, `merge`
samt Tests) **komplett entfernen** — die DP löst jede Instanzgröße direkt.
`PageAssignmentSolver::solve` schrumpft auf:

```rust
pub fn solve(&self, groups: &GroupInfo, photos: &[Photo]) -> Result<PageAssignment, dp::DpError> {
    dp::solve_dp(groups, &self.params).or_else(|err| {
        info!("DP infeasible ({err}), using heuristic start solution");
        Ok(create_start_solution::create_start_solution(&self.params, photos))
    })
}
```

Fallback-Verhalten wie bisher beibehalten (Heuristik statt Fehler). Alternativ darf
die Datei ganz entfallen und `solve_book_layout` ruft `dp::solve_dp` + Fallback
direkt auf — dann `create_start_solution`-Import dorthin ziehen. Hinweis:
`create_start_solution` wird **nicht** mehr als Warm-Start-Hint gebraucht, nur noch
als Fallback.

### Löschen

- `src/solver/book_layout_solver/mip.rs` und Ordner `src/solver/book_layout_solver/mip/`.
- Abhängigkeit `good_lp` aus `Cargo.toml` (vorher per grep prüfen, dass kein
  weiterer Nutzer existiert; Stand heute: nur die MIP-Module).

### Config-Felder entfernen

In `src/dto_models/config/book_layout_solver_config.rs` (Struct, Defaults,
Validierung, serde) die Felder `mip_rel_gap`, `max_photos_for_split`,
`split_group_boundary_slack` entfernen. `search_timeout` **bleibt** (Deadline der
Local Search, `local_search/improve.rs`).

Danach projektweit nach den Feldnamen greppen und alle Verwendungen bereinigen:
Test-Fixtures (`mip.rs`-Tests entfallen, außerdem `model.rs`,
`page_assignment_solver.rs`, `local_search/improve.rs`,
`local_search/perturbation.rs`, `book_layout_solver.rs`), CLI-/Beispiel-Configs
und `docs/book/src/general/solver-tuning.md` (MIP-Abschnitte durch DP ersetzen).

### Doku

In `docs/design/book_layout_solver_mip/` einen Hinweis ergänzen, dass die
Formulierung durch `book_layout_solver_dp/` abgelöst ist (Dokument nicht löschen,
die Äquivalenz-Argumentation in `dp.typ` referenziert es).

## Tests (`#[cfg(test)]` in `dp.rs`)

1. **Portierung**: Alle Tests aus `mip.rs` übernehmen (einfache Instanzen,
   `g_min`-Respektierung, die vier Gewichts-Isolationstests, Even-vs-Split-Tradeoff).
   Erwartungen bleiben identisch, da die Zielfunktion identisch ist. Die Tests
   laufen ohne Solver-Timeout in Millisekunden — kein `slow-tests`-Feature nötig.
2. **Exaktheit per Brute-Force**: Für kleine Instanzen (n ≤ 12, 2–3 Gruppen) alle
   Cut-Vektoren enumerieren, Kosten gemäß Z aus `dp.typ` berechnen und prüfen,
   dass `solve_dp` denselben Optimalwert liefert (Wert vergleichen, nicht Cuts —
   Optima können mehrdeutig sein).
3. **Unzulässigkeit**: z. B. unspaltbare Gruppe größer als `photos_per_page_max`
   → `Err(Infeasible)`.
4. **Determinismus**: zwei Aufrufe liefern identische Cuts.
5. **Performance**: 1000 Bilder, ~30 Gruppen, `page_max` ≈ 100: assert Laufzeit
   < 1 s (normaler Test, kein Benchmark-Harness nötig).
6. **Cut-Kosten-Semantik**: Cut exakt auf Gruppengrenze kostet kein `weight_split`
   (Instanz, deren Optimum genau das ausnutzt).

## Commit-Plan (Conventional Commits, je Teilschritt)

1. `feat(solver): add exact DP page assignment solver` — `dp.rs` + Tests,
   noch ungenutzt.
2. `refactor(solver): replace MIP page assignment with DP` — Integration,
   MIP-Module und Splitting entfernen, `good_lp` raus.
3. `refactor(config): drop obsolete MIP solver config fields` — Config, Fixtures,
   solver-tuning.md.

Vor jedem Commit: `cargo build` (warnungsfrei), `cargo clippy --fix`, `cargo fmt`.

## Akzeptanzkriterien

- Alle bestehenden Tests grün (abzüglich gelöschter MIP-/Splitting-Tests,
  deren Pendants nach `dp.rs` portiert sind).
- Brute-Force-Vergleichstest bestätigt Exaktheit.
- 1000 Bilder / 30 Gruppen in < 1 s, deterministisch.
- `good_lp`/HiGHS nicht mehr im Dependency-Tree.
