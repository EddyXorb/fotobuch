# Book Layout Solver — Implementierungsplan

## Überblick

Der Solver verteilt eine sortierte Sequenz von Bildern (gruppiert, chronologisch) auf Buchseiten. Er besteht aus zwei Phasen:

1. **MIP-Solver:** Findet eine global optimale Seitenzuteilung unter strukturellen Constraints (Gruppenordnung, Seitengrößen, Spaltungsregeln).
2. **Local Search:** Verbessert die Zuteilung iterativ basierend auf der tatsächlichen Layout-Qualität des nachgelagerten Page-Layout-Solvers.

Ein **Layout-Cache** verbindet beide Phasen und vermeidet redundante Berechnungen.

---

## Modul-Architektur

```
solver/
├── book_layout_solver.rs          // Öffentliche API: solve()
├── book_layout_solver/
│   ├── model.rs                // PageAssignment, Params, Cost-Typen
│   ├── mip.rs                  // MIP-Formulierung, HiGHs-Aufruf
│   ├── local_search.rs         // Nachbarschaftssuche (generisch + konkret)
│   ├── cache.rs                // Layout-Cache
│   └── feasibility.rs          // Constraint-Prüfung auf PageAssignment
```

`book_layout_solver` hat eine Abhängigkeit auf den bestehenden `page_layout_solver` (konkret: `run_ga` → `GaResult` mit `CostBreakdown`), aber nur über das `PageLayoutEvaluator`-Trait. Die konkrete Implementierung `GaPageLayoutEvaluator` liegt in `book_layout_solver.rs` und kapselt die Abhängigkeit. Änderung am bestehenden Code: `run_ga` gibt `GaResult` statt Tupel zurück und re-exportiert `CostBreakdown`.

---

## Datenmodell (`model.rs`)

### `Params`

Alle konfigurierbaren Eingabeparameter, entsprechend der MIP-Formulierung:

```rust
struct Params {
    page_target: usize,         // s
    page_min: usize,            // b_min
    page_max: usize,            // b_max
    photos_per_page_min: usize, // p_min
    photos_per_page_max: usize, // p_max
    group_max_per_page: usize,  // g_max
    group_min_photos: usize,    // g_min
    // Zielfunktions-Gewichte (MIP)
    weight_even: f64,           // w_1
    weight_split: f64,          // w_2
    weight_pages: f64,          // w_3
    // Local Search
    search_timeout: Duration,
    max_coverage_cost: f64,     // Seiten mit coverage > diesem Wert gelten als "schlecht"
}
```

Validierung in `Params::validate()`: prüft `p_min >= g_min`, `b_min <= b_max`, `n` passt in `[b_min * p_min, b_max * p_max]`, etc. Gibt `Result<(), ValidationError>` zurück.

### `PageAssignment`

Ergebnis der Seitenzuteilung — eine Partitionierung der Bildsequenz in Seiten:

```rust
struct PageAssignment {
    /// Schnittpunkte: Seite j enthält Bilder [cuts[j-1]..cuts[j]).
    /// cuts[0] = 0, cuts[last] = n. Länge = Seitenanzahl + 1.
    cuts: Vec<usize>,
}
```

Abgeleitete Methoden:
- `num_pages() -> usize`
- `page_size(j) -> usize` — Anzahl Bilder auf Seite j
- `page_range(j) -> Range<usize>` — Bildindizes für Seite j
- `affected_pages(cut_index) -> (usize, usize)` — die beiden Seiten links/rechts eines Schnittpunkts

Kollabierte Schnittpunkte (aus dem MIP, wo `cuts[j] == cuts[j+1]`) werden bei der Konvertierung aus dem MIP-Ergebnis entfernt, sodass `PageAssignment` immer nur aktive Seiten enthält.

### `GroupInfo`

```rust
struct GroupInfo {
    /// Kumulative Gruppengrößen: group_ends[0] = |G_1|, group_ends[1] = |G_1|+|G_2|, ...
    group_ends: Vec<usize>,
}
```

Methoden: `num_groups()`, `group_size(l)`, `group_of_photo(i)`, `group_range(l)`.

### Cost (Seitenbewertung)

Direkt abgeleitet vom `cost_breakdown` des bestehenden `page_layout_solver::fitness`-Moduls.

```rust
/// Cost einer einzelnen Seite. Niedriger = besser.
/// Felder entsprechen den Termen aus `fitness::cost_breakdown`.
struct PageCost {
    total: f64,
    size: f64,
    coverage: f64,     // Primäres Kriterium für "Seite ist schlecht"
    barycenter: f64,
    order: f64,
}

/// Cost der gesamten Zuteilung.
struct AssignmentCost {
    page_costs: Vec<PageCost>,
    worst: f64,       // max(coverage) über alle Seiten (primäres Kriterium)
    average: f64,     // Durchschnitt coverage (Tiebreaker)
    worst_page: usize,
}
```

`AssignmentCost` implementiert `Ord`: Vergleich primär nach `worst` (niedriger = besser), sekundär nach `average`. Verwendet `coverage` als Vergleichsgröße, da diese den sichtbaren Weißraum abbildet.

---

## Constraint-Prüfung (`feasibility.rs`)

Eine reine Funktion, die prüft ob ein `PageAssignment` alle Hard-Constraints erfüllt:

```rust
fn check_feasibility(
    assignment: &PageAssignment,
    groups: &GroupInfo,
    params: &Params,
) -> Result<(), ConstraintViolation>
```

Prüft in dieser Reihenfolge (early return bei Verletzung):
1. Seitenanzahl in `[b_min, b_max]`
2. Jede Seitengröße in `[p_min, p_max]`
3. Max. Gruppen pro Seite ≤ `g_max`
4. `g_min`-Regel: Wenn Gruppe l auf Seite j nicht komplett ist und `|G_l| >= g_min`, dann `n_{l,j} >= g_min`
5. Sequentielle Ordnung (folgt automatisch aus der Schnittpunkt-Darstellung)

`ConstraintViolation` ist ein Enum mit Varianten pro Constraint-Typ, inklusive der betroffenen Seite/Gruppe für Debugging.

Diese Funktion wird sowohl nach dem MIP-Solve als auch nach jeder Perturbation in der Local Search aufgerufen.

---

## MIP-Solver (`mip.rs`)

### Schnittstelle

```rust
fn solve_mip(
    groups: &GroupInfo,
    params: &Params,
) -> Result<PageAssignment, MipError>
```

`MipError`: `Infeasible`, `Timeout`, `SolverError(String)`.

### Implementierung

Verwendet `good_lp` mit HiGHs-Backend. Die Formulierung entspricht dem Typst-Dokument `page_assignment_mip.typ`.

Konkrete Hinweise:
- `good_lp::variable()` für `g_{l,j}` (Integer, bounds `0..=group_size`), `b_{l,j}`, `w_{l,j}`, `a_j` (Binary), `d_j`, `d_s` (Continuous, non-negative).
- `n_{l,j}` wird nicht als Variable angelegt, sondern als Expression `g[l][j] - g[l][j-1]` inline verwendet.
- Constraints werden in separaten Funktionen aufgebaut (eine pro Abschnitt im Typst-Dokument), die jeweils `&mut ProblemVariables` und den `Solver` nehmen.
- Nach dem Solve: Schnittpunkte aus den kumulativen `g_{l,j}`-Werten extrahieren. Seite j hat `sum_l n_{l,j}` Bilder. Schnittpunkte = kumulative Summe der Seitengrößen.
- Inaktive Seiten (wo `a_j = 0` bzw. Seitengröße = 0) beim Aufbau von `PageAssignment` überspringen.

### Rust-Hinweis: `good_lp`-Variablen-Layout

`good_lp` arbeitet mit `Variable`-Handles. Für N-dimensionale Variablen (1D für `a_j`, 2D für `g_{l,j}`, `b_{l,j}`, ...) eine generische `VarMap` mit `HashMap`-Backend:

```rust
struct VarMap<const N: usize> {
    vars: HashMap<[usize; N], Variable>,
}

impl<const N: usize> VarMap<N> {
    fn new() -> Self { Self { vars: HashMap::new() } }
    fn insert(&mut self, index: [usize; N], var: Variable) { self.vars.insert(index, var); }
    fn get(&self, index: [usize; N]) -> Variable { self.vars[&index] }
    fn iter(&self) -> impl Iterator<Item = (&[usize; N], &Variable)> { self.vars.iter() }
}
```

Vorteil: Lücken sind natürlich darstellbar — `w_{l,j}` wird nur für spaltbare Gruppen angelegt:

```rust
let mut g: VarMap<2> = VarMap::new();
for l in 0..k {
    for j in 0..=b_max {
        g.insert([l, j], problem.add(variable().integer().min(0).max(group_sizes[l])));
    }
}

let mut w: VarMap<2> = VarMap::new();
for l in splittable_groups() {
    for j in 0..b_max {
        w.insert([l, j], problem.add(variable().binary()));
    }
}
```

---

## Layout-Cache (`cache.rs`)

### Zweck

Speichert das beste bekannte Layout-Ergebnis für eine gegebene Bildmenge auf einer Seite. Verhindert redundante Layout-Solver-Aufrufe und stellt Monotonie sicher (Ergebnisse werden nur besser).

### Schlüssel

```rust
/// Identifiziert eine Seitenbelegung eindeutig.
/// Da die Bilder sortiert sind, reicht der Range.
#[derive(Hash, Eq, PartialEq)]
struct CacheKey {
    photo_range: Range<usize>, // Start- und End-Index in der globalen Bildliste
}
```

### Struktur

```rust
struct LayoutCache {
    entries: HashMap<CacheKey, PageCost>,
}

impl LayoutCache {
    /// Gibt gecachte Cost zurück, falls vorhanden.
    fn get(&self, range: Range<usize>) -> Option<&PageCost>;

    /// Speichert nur, wenn besser (niedrigerer coverage-Cost) als vorhandener Eintrag.
    /// Gibt true zurück, wenn der Eintrag aktualisiert wurde.
    fn insert_if_better(&mut self, range: Range<usize>, cost: PageCost) -> bool;
}
```

---

## Schnittstelle zum Layout-Solver

Der Book-Layout-Solver kennt den Page-Layout-Solver nur über ein Trait:

```rust
trait PageLayoutEvaluator {
    fn evaluate(&mut self, photos: &[Photo]) -> PageCost;
}
```

Die konkrete Implementierung delegiert an `page_layout_solver::run_ga`. Da `fitness::cost_breakdown` ein privates Submodul von `page_layout_solver` ist, muss `run_ga` das Breakdown mit zurückgeben. Dazu wird die Signatur erweitert:

```rust
// Bestehende Signatur (page_layout_solver.rs):
//   pub(super) fn run_ga(...) -> (SlicingTree, PageLayout, f64)
//
// Neue Signatur:
pub(super) fn run_ga(
    photos: &[Photo],
    canvas: &Canvas,
    ga_config: &GaConfig,
) -> GaResult

pub(super) struct GaResult {
    pub tree: SlicingTree,
    pub layout: PageLayout,
    pub cost: f64,
    pub breakdown: CostBreakdown, // bisher nur intern für Logging verwendet
}
```

`CostBreakdown` entspricht dem bestehenden Return-Typ von `fitness::cost_breakdown` und wird aus `page_layout_solver` re-exportiert.

```rust
struct GaPageLayoutEvaluator<'a> {
    canvas: &'a Canvas,
    ga_config: &'a GaConfig,
}

impl PageLayoutEvaluator for GaPageLayoutEvaluator<'_> {
    fn evaluate(&mut self, photos: &[Photo]) -> PageCost {
        let result = page_layout_solver::run_ga(photos, self.canvas, self.ga_config);
        PageCost {
            total: result.breakdown.total,
            size: result.breakdown.size,
            coverage: result.breakdown.coverage,
            barycenter: result.breakdown.barycenter,
            order: result.breakdown.order,
        }
    }
}
```

In der Local Search wird der Evaluator **immer über den Cache** aufgerufen. Der Aufrufer sliced `&photos[range]` vor dem Aufruf:

```rust
fn evaluate_cached(
    evaluator: &mut impl PageLayoutEvaluator,
    cache: &mut LayoutCache,
    photos: &[Photo],
    range: Range<usize>,
) -> PageCost {
    if let Some(cached) = cache.get(range.clone()) {
        return cached.clone();
    }
    let cost = evaluator.evaluate(&photos[range.clone()]);
    cache.insert_if_better(range, cost.clone());
    cost
}
```

---

## Local Search (`local_search.rs`)

### Algorithmus

Gerichtete Nachbarschaftssuche (Variable Neighborhood Search, VNS-Variante):

```
Eingabe: assignment (aus MIP), photos, groups, params, evaluator, timeout
Cache initialisieren

1. Initiale Bewertung: Für jede Seite j Layout berechnen und cachen.
   → AssignmentCost berechnen.

2. Solange Zeit übrig:
   a. Kandidaten-Schnittpunkte bestimmen:
      Alle cuts[j], bei denen mindestens eine Nachbarseite
      coverage > max_coverage_cost hat.
      Falls keine: fertig (alle Seiten gut genug).

   b. Einen Kandidaten wählen (worst-first).

   c. Perturbation anwenden:
      Verschiebe cuts[j] um delta ∈ {-1, +1, -2, +2, ...}
      (aufsteigend nach |delta|).
      Für jedes delta:
        - Neues Assignment konstruieren
        - check_feasibility() → bei Verletzung: skip
        - Betroffene Seiten (j-1, j) evaluieren (via Cache)
        - Falls AssignmentCost besser (worst↓, dann average↓): akzeptieren, break
      Falls kein delta Verbesserung bringt: Kandidat überspringen.

   d. Nächste Iteration.

3. Bestes gefundenes Assignment + Costs zurückgeben.
```

### Perturbationslogik im Detail

```rust
fn try_perturbation(
    assignment: &PageAssignment,
    cut_index: usize,        // welcher Schnittpunkt
    delta: i32,              // Verschiebung
    groups: &GroupInfo,
    params: &Params,
) -> Option<PageAssignment>
```

Gibt `None` zurück bei Constraint-Verletzung (Feasibility-Check), sonst das neue Assignment. Der Aufrufer bewertet dann die betroffenen Seiten.

### Maximale Perturbationsgröße

`|delta|` wird begrenzt durch:
- `p_max - p_min` (sinnlose Verschiebungen vermeiden)
- Konkret: `max_delta = (p_max - p_min) / 2`, mindestens 2

### Worst-First-Selektion

Kandidaten werden nach Coverage-Cost der schlechtesten Nachbarseite sortiert (absteigend). Dadurch werden die problematischsten Grenzen zuerst angegangen.

### Rust-Hinweis: Zeitsteuerung

```rust
let deadline = Instant::now() + params.search_timeout;
while Instant::now() < deadline {
    // ...
}
```

---

## Öffentliche API (`book_layout_solver.rs`)

```rust
pub fn solve(
    photos: &[Photo],
    groups: &GroupInfo,
    params: &Params,
    evaluator: &mut impl PageLayoutEvaluator,
) -> Result<SolverResult, SolverError>
```

```rust
pub struct SolverResult {
    pub assignment: PageAssignment,
    pub cost: AssignmentCost,
    pub iterations: usize,
    pub cache_hits: usize,
}
```

Ablauf:
1. `params.validate()?`
2. `mip::solve_mip(groups, params)?` → initiales Assignment
3. `local_search::improve(assignment, photos, groups, params, evaluator)` → optimiertes Assignment
4. `SolverResult` zusammenbauen

---

## Teststrategie

### Unit-Tests

- **`feasibility.rs`**: Handkonstruierte Assignments, die jeweils genau einen Constraint verletzen. Sicherstellen, dass die richtige `ConstraintViolation`-Variante zurückkommt.
- **`model.rs`**: `PageAssignment`-Methoden (`page_size`, `page_range`, `affected_pages`).
- **`cache.rs`**: `insert_if_better` überschreibt nur bei niedrigerem Coverage-Cost.
- **`mip.rs`**: Kleine Instanzen (3 Gruppen, 5 Seiten) mit bekannter optimaler Lösung.

### Integrationstests

- **Mock-Evaluator**: Implementiert `PageLayoutEvaluator` mit deterministischer Cost-Berechnung (z.B. Coverage-Cost proportional zur Abweichung der Bildanzahl von einem Idealwert). Damit lässt sich die Local Search testen, ohne den echten GA-Layout-Solver.
- **Roundtrip**: MIP → Local Search → Feasibility-Check auf Endergebnis.

### Property-Tests (`proptest`)

- Jedes von `solve()` zurückgegebene Assignment erfüllt alle Hard-Constraints.
- `PageAssignment::cuts` ist streng monoton steigend, beginnt bei 0, endet bei n.
- Gesamtbildzahl bleibt erhalten: `cuts.last() == n`.

---

## Umsetzungsreihenfolge

| Schritt | Modul | Beschreibung |
|---------|-------|-------------|
| 1 | `page_layout_solver.rs` | Refactor: `run_ga` → `GaResult`, `CostBreakdown` re-exportieren |
| 2 | `model.rs` | Datentypen, `Params::validate()`, `PageAssignment`-Methoden |
| 3 | `feasibility.rs` | Constraint-Prüfung mit Tests |
| 4 | `cache.rs` | `LayoutCache` mit Tests |
| 5 | `mip.rs` | MIP-Formulierung, HiGHs-Integration, Tests mit kleinen Instanzen |
| 6 | `local_search.rs` | Nachbarschaftssuche mit Mock-Evaluator |
| 7 | `book_layout_solver.rs` | Öffentliche API, `GaPageLayoutEvaluator`, Integration |
| 8 | Integration | End-to-End-Tests mit echtem Page-Layout-Solver |

Schritt 1 ist ein kleiner, isolierter Refactor am bestehenden Code. Schritte 2–4 haben keine externe Abhängigkeit und können parallel entwickelt werden. Schritt 5 benötigt `good_lp` + HiGHs als Dependency. Schritt 6 benötigt 2–4.
