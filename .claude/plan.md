# Projektplan: Slicing-Tree Photo Layout Solver

Implementierungsplan für den Slicing-Tree GA Layout Solver in Rust.
Algorithmus-Details siehe `slicing_tree_ga_algorithm.md`.

Nach jedem schritt ein commit machen mit beschreibung in knappen worten, mit conventional commits.

## Projektstruktur

```
photobook-layout/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs                         # Re-exports, zentrale Typen
│   │
│   ├── input/                         # --- Alles was Daten einliest ---
│   │   ├── input.rs                   # Re-exports
│   │   ├── cli.rs                     # clap-Definitionen, Argument-Parsing
│   │   ├── exif.rs                    # EXIF-Parsing (Aspect-Ratio, Timestamp, Rotation)
│   │   └── manifest.rs               # JSON-Manifest lesen (Alternative zu Verzeichnis-Scan)
│   │
│   ├── model/                         # --- Domänen-Typen, kein Verhalten ---
│   │   ├── model.rs                   # Re-exports
│   │   ├── photo.rs                   # Photo { aspect_ratio, area_weight, group, timestamp }
│   │   ├── canvas.rs                  # Canvas { width, height, beta, bleed }
│   │   ├── layout.rs                  # LayoutResult, PhotoPlacement { photo_idx, x, y, w, h }
│   │   └── weights.rs                 # FitnessWeights { w_size, w_coverage, w_bary, w_order }
│   │
│   ├── solver/                        # --- Kern-Algorithmus ---
│   │   ├── solver.rs                  # Re-exports, solve()-Einstiegspunkt
│   │   ├── tree.rs                    # SlicingTree, Node, Cut — Arena-Datentypen
│   │   ├── tree/
│   │   │   ├── build.rs              # random_tree()
│   │   │   ├── operators.rs          # mutate(), crossover()
│   │   │   └── validate.rs           # Invarianten-Checks
│   │   ├── layout_solver.rs           # Affiner Solver: Koeffizienten + Dimensionen
│   │   ├── fitness.rs                 # Kostenfunktion: C1, C2, C_bary, C_order
│   │   ├── ga.rs                      # GA-Loop, Selektion, Elitismus
│   │   └── ga/
│   │       └── island.rs             # Island Model, Migration, Threading
│   │
│   └── output/                        # --- Alles was Ergebnisse schreibt ---
│       ├── output.rs                  # Re-exports
│       ├── json.rs                    # JSON-Export
│       └── typst.rs                   # Typst-Export (place()-Aufrufe)
│
└── tests/
    ├── solver_integration.rs          # End-to-End: Photos → Layout
    ├── ga_convergence.rs              # GA verbessert Fitness monoton
    └── fixtures/
        └── test_photos.json           # Testdaten
```

**Modul-Konvention:** `foo.rs` + `foo/` Verzeichnis (kein `mod.rs`). `foo.rs` definiert die öffentlichen Typen und re-exportiert Submodule.

**Verantwortlichkeiten:**

| Modul | Verantwortung | Abhängigkeiten |
|---|---|---|
| `input/` | Daten einlesen, CLI parsen | → `model/` |
| `model/` | Reine Datentypen, kein Verhalten | keine |
| `solver/` | Baum, Layout-Berechnung, GA | → `model/` |
| `output/` | Ergebnisse serialisieren | → `model/` |

`solver/` kennt `input/` und `output/` nicht. `main.rs` verdrahtet die Module.

## Datentyp-Entscheidungen

### Node: `Option<u16>` statt Sentinel

```rust
#[derive(Clone, Copy)]
enum Cut { V, H }

#[derive(Clone, Copy)]
enum Node {
    Leaf {
        photo_idx: u16,
        parent: Option<u16>,
    },
    Internal {
        cut: Cut,
        left: u16,
        right: u16,
        parent: Option<u16>,
    },
}
```

Root hat `parent: None`. Kein Sentinel-Wert (`u16::MAX`), expliziter Typ.

**Hinweis:** `Option<u16>` ist 4 Bytes (2 Bytes Wert + 2 Bytes Discriminant, aligned). Gesamter Node bleibt ≤8 Bytes, Copy-günstig.

### FitnessWeights: Null-Koeffizienten überspringen

```rust
struct FitnessWeights {
    w_size: f64,       // C1
    w_coverage: f64,   // C2
    w_bary: f64,       // C_bary
    w_order: f64,      // C_order
}
```

Die `total_cost()`-Funktion prüft jeden Koeffizienten vor der Berechnung:

```rust
fn total_cost(layout: &LayoutResult, photos: &[Photo], canvas: &Canvas, w: &FitnessWeights) -> f64 {
    let mut cost = 0.0;
    if w.w_size != 0.0 {
        cost += w.w_size * cost_size_distribution(layout, photos, canvas);
    }
    if w.w_coverage != 0.0 {
        cost += w.w_coverage * cost_coverage(layout, canvas);
    }
    if w.w_bary != 0.0 {
        cost += w.w_bary * cost_barycenter(layout, canvas);
    }
    if w.w_order != 0.0 {
        cost += w.w_order * cost_reading_order(layout, photos);
    }
    cost
}
```

Bei hunderttausenden Evaluierungen pro GA-Lauf spart das messbar Zeit, wenn Terme deaktiviert sind. Jede `cost_*`-Funktion bleibt aber unabhängig aufrufbar für Tests und Debugging.

## CLI-Parameter

```
photobook-layout [OPTIONS] <INPUT>

ARGUMENTE:
    <INPUT>                Pfad zum Foto-Verzeichnis oder JSON-Manifest

CANVAS:
    --width <mm>           Canvas-Breite in mm (Pflicht)
    --height <mm>          Canvas-Höhe in mm (Pflicht)
    --beta <mm>            Gap zwischen Fotos [default: 2.0]
    --bleed <mm>           Bleed über Papierrand [default: 3.0]

SOLVER:
    --islands <n>          Anzahl Islands [default: Anzahl CPU-Kerne]
    --population <n>       Population pro Island [default: 300]
    --generations <n>      Max. Generationen [default: 100]
    --migration-interval <n>  Generationen zwischen Migrationen [default: 5]
    --migrants <n>         Migranten pro Migration [default: 2]
    --timeout <secs>       Max. Gesamtlaufzeit [default: 30]
    --seed <u64>           RNG-Seed für Reproduzierbarkeit [optional]

GEWICHTE:
    --w-size <f>           Gewicht Größenverteilung C1 [default: 1.0]
    --w-coverage <f>       Gewicht Canvas-Abdeckung C2 [default: 0.15]
    --w-barycenter <f>     Gewicht Baryzentrum C_bary [default: 0.5]
    --w-order <f>          Gewicht Leseordnung C_order [default: 0.3]

FOTOS:
    --area-weight <idx:weight>  Area-Weight für Foto (wiederholbar) [default: 1.0]

OUTPUT:
    --output <path>        Ausgabedatei [default: layout.json]
    --format <fmt>         json | typst [default: json]
    --verbose              Fortschritt und Fitness pro Generation auf stderr
```

## Implementierungsschritte

Jeder Schritt ist ein abgeschlossener, testbarer Meilenstein. TDD: Tests zuerst, dann Implementierung.

### Schritt 1: Domänen-Typen (`model/`)

**Dateien:** `model/photo.rs`, `model/canvas.rs`, `model/layout.rs`, `model/weights.rs`

**Implementieren:**
- `Photo { aspect_ratio: f64, area_weight: f64, group: String, timestamp: Option<DateTime<Utc>> }`
- `Canvas { width: f64, height: f64, beta: f64, bleed: f64 }` mit `area()` Methode
- `PhotoPlacement { photo_idx: u16, x: f64, y: f64, w: f64, h: f64 }`
- `LayoutResult { placements: Vec<PhotoPlacement>, canvas: Canvas }`
- `FitnessWeights` mit Default-Impl für Standardgewichte

**Tests:**
- Canvas-Area korrekt
- FitnessWeights::default() liefert dokumentierte Werte
- PhotoPlacement: `center()`, `area()` Hilfsmethoden

### Schritt 2: Baum-Datenstruktur und Aufbau (`solver/tree*`)

**Dateien:** `solver/tree.rs`, `solver/tree/build.rs`, `solver/tree/validate.rs`

**Implementieren:**
- `Node`, `Cut`, `SlicingTree` mit Arena-Vec
- `random_tree(n, rng)` → `SlicingTree`
- `validate_tree()` prüft:
  - Genau N Blätter, N−1 innere Knoten
  - Jedes `photo_idx` kommt genau einmal vor (Permutation von 0..N)
  - Alle `left`/`right` zeigen auf gültige Indizes
  - Alle `parent`-Referenzen konsistent, Root hat `parent: None`

**Tests:**
- Baum mit N=2: genau 3 Knoten, Root ist Internal
- 1000 zufällige Bäume für N=2..30: alle bestehen `validate_tree()`
- `leaf_count()` == N für alle erzeugten Bäume
- Clone erzeugt identischen, unabhängigen Baum

### Schritt 3: Layout Solver (`solver/layout_solver.rs`)

**Dateien:** `solver/layout_solver.rs`

**Implementieren:**
- `AffineCoeff { alpha: f64, gamma: f64 }`
- `compute_coefficients(tree, photos, beta)` → `Vec<AffineCoeff>` (bottom-up)
- `compute_dimensions(tree, coeffs, canvas)` → `Vec<(f64, f64)>` (top-down, w/h pro Knoten)
- `compute_positions(tree, dims, beta)` → `Vec<(f64, f64)>` (top-down, x/y pro Knoten)
- `solve_layout(tree, photos, canvas)` → `LayoutResult` (kombiniert alle drei Schritte)

**Tests:**
- 2 Fotos, V, β=0: `w1 + w2 ≈ canvas.width`
- 2 Fotos, H, β=0: `h1 + h2 ≈ canvas.height`
- 2 Fotos, V, β>0: `w1 + w2 + β ≈ canvas.width`
- 3 Fotos, H-über-V, β=0.5: Handgerechnetes Beispiel aus Algorithmus-Dokument (a1=3, a2=1/3, a3=1)
- β=0 Ergebnis identisch mit klassischem Aspect-Ratio-Solver (Gegenprüfung)
- Property-Tests: 1000 zufällige Bäume, N=3..20, β∈[0,5]:
  - Alle Dimensionen positiv
  - Kein Foto ragt über Canvas hinaus
  - Keine Überlappung (Rechteck-Schnitt-Test)
  - Gaps zwischen Nachbarfotos ≈ β (Toleranz 1e-9)

### Schritt 4: Kostenfunktion (`solver/fitness.rs`)

**Dateien:** `solver/fitness.rs`

**Implementieren:**
- `cost_size_distribution(layout, photos, canvas)` → f64
- `cost_coverage(layout, canvas)` → f64
- `cost_barycenter(layout, canvas)` → f64
- `cost_reading_order(layout, photos)` → f64
- `total_cost(layout, photos, canvas, weights)` → f64, überspringt Terme mit Gewicht 0.0

**Tests:**
- C1 = 0 wenn alle Fotos exakt in Wunschgröße
- C1: Foto mit `s_i/t_i < 0.5` bekommt k_i=5
- C2 = 0 bei 100% Abdeckung
- C_bary ≈ 0 bei symmetrischem Layout
- C_order = 0 wenn Fotos in Leseordnung
- `total_cost` mit `w_bary=0.0`: `cost_barycenter` wird nicht aufgerufen (Mock/Counter oder Zeitmessung)
- `total_cost` mit allen Gewichten 0.0 → Ergebnis 0.0

### Schritt 5: Genetische Operatoren (`solver/tree/operators.rs`)

**Dateien:** `solver/tree/operators.rs`

**Implementieren:**
- `mutate(tree, rng)`: Zwei Knoten gleichen Typs, Labels tauschen
- `crossover(a, b, rng) -> Option<(SlicingTree, SlicingTree)>`: Teilbäume gleicher Blattanzahl ≥3 tauschen
- Hilfsfunktionen: `subtree_leaf_counts(tree)`, `extract_subtree()`, `graft_subtree()`

**Tests:**
- Mutation: Baum bleibt valide, Struktur unverändert, genau zwei Labels getauscht
- Mutation N=2: Blatt-Labels getauscht
- Crossover: Beide Ergebnis-Bäume valide, Blattanzahl unverändert
- Crossover inkompatibel: → `None`
- 1000x Mutation + Crossover auf zufälligen Bäumen → alle `validate_tree()`

### Schritt 6: GA-Loop Single-Thread (`solver/ga.rs`)

**Dateien:** `solver/ga.rs`

**Implementieren:**
- `GaConfig { population: usize, generations: usize, mutation_rate: f64, crossover_rate: f64, tournament_size: usize, elitism_ratio: f64, timeout: Duration }`
- `run_ga(photos, canvas, weights, config, rng)` → `(SlicingTree, LayoutResult, f64)`
- Tournament Selection (Größe 3–5)
- Elitismus: Top 5% unverändert übernehmen
- Abbruch: Max. Generationen ODER Timeout ODER Plateau (keine Verbesserung seit k Generationen)

**Tests:**
- GA mit N=3, 10 Generationen: Fitness sinkt (Elitismus → beste Fitness monoton fallend)
- Identische Fotos (a=1, t=1) → nahezu perfekte Abdeckung
- Ergebnis ist valider Baum mit validem Layout
- Gleicher Seed → identisches Ergebnis

### Schritt 7: Island Model (`solver/ga/island.rs`)

**Dateien:** `solver/ga/island.rs`

**Implementieren:**
- `IslandConfig { islands: usize, migration_interval: usize, migrants: usize }`
- `run_island_ga(photos, canvas, weights, ga_config, island_config)` → `(SlicingTree, LayoutResult, f64)`
- Jede Island als Thread via `std::thread::scope`
- Migration über `crossbeam::channel` (Sender/Receiver-Paar pro Island-Paar)
- Globaler Abbruch via `AtomicBool`

**Tests:**
- Island-GA produziert valides Ergebnis
- 1 Island = identisch zu Single-Thread GA (gleicher Seed)
- 4 Islands: gleiche oder bessere Lösung als Single-Thread
- Timeout eingehalten (±100ms)

### Schritt 8: Input und Output (`input/`, `output/`)

**Dateien:** `input/cli.rs`, `input/exif.rs`, `input/manifest.rs`, `output/json.rs`, `output/typst.rs`

**Implementieren:**
- CLI mit `clap` derive API
- Verzeichnis scannen → EXIF lesen → `Vec<Photo>` sortiert (Gruppe lex., Timestamp)
- EXIF-Rotation beachten (Landscape/Portrait → Aspect-Ratio anpassen)
- JSON-Manifest als Alternative zu Verzeichnis-Scan
- JSON-Export: `{ photos: [{ idx, x, y, w, h, path }], canvas: { w, h }, beta }`
- Typst-Export: `place(dx: ..mm, dy: ..mm)` Aufrufe

**Tests:**
- CLI parst alle Parameter, Defaults korrekt
- EXIF-Rotation wird berücksichtigt
- JSON-Export valide und re-parsebar
- Typst-Export syntaktisch korrekt

### Schritt 9: Integration und Kalibrierung

**Kein neuer Code:**
- End-to-End: 10 Testfotos → JSON → visuell prüfen
- Gewichte kalibrieren anhand Testdaten
- Affinen Solver gegen handgerechnete Fälle verifizieren
- Performance-Benchmark: 25 Fotos, 8 Islands, 100 Generationen — Ziel: <5s

## Workflow-Konventionen

### TDD-Zyklus

1. Test schreiben der fehlschlägt
2. Minimale Implementierung die den Test besteht
3. Refactoren
4. Nächster Test

### Code-Stil

- Funktionen ≤30 Zeilen, eine klare Aufgabe
- Typ-Annotationen an allen `pub`-Funktionen
- Docstrings (`///`) an allen `pub`-Items
- Inline-Kommentare nur wo der Code nicht selbsterklärend ist
- `clippy::pedantic` aktivieren
- `#[must_use]` an Funktionen die Werte zurückgeben

### Test-Coverage

- Ziel: >90% Line-Coverage
- Tool: `cargo-llvm-cov`
- Jedes Modul hat Unit-Tests (`#[cfg(test)] mod tests`)
- Integrationstests in `tests/`
- Property-based Tests mit `proptest` für Baum-Invarianten und Solver-Korrektheit

### Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
rand = "0.8"
rayon = "1"
crossbeam = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
kamadak-exif = "0.5"
chrono = "0.4"

[dev-dependencies]
proptest = "1"
approx = "0.5"
```

### Umsetzungsreihenfolge

```
Schritt 1 (model) → 2 (tree) → 3 (solver) → 4 (fitness) → 5 (operators) → 6 (ga)
                                                                                │
                                                                    ┌───────────┼───────────┐
                                                                    ▼           ▼           ▼
                                                              7 (island)   8 (i/o)    9 (integration)
```

Erste lauffähige Version nach Schritt 6. Schritte 7, 8 sind unabhängig voneinander.