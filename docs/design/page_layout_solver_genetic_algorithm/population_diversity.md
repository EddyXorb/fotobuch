# Genpool-Diversität: Duplikat-Eliminierung

## Problem

Elitismus + Tournament-Selection + fehlende Diversitätskontrolle führt zu schneller Konvergenz: dominante Individuen übernehmen die Population. Besonders kritisch mit `enforce_order`, da der Suchraum kleiner wird (nur Topologie + Cuts, keine Fotozuweisung).

Aktuell: Kein `PartialEq`/`Hash` auf `SlicingTree`, kein Duplikat-Check.

## Ansatz

Nach Selection, vor Crossover/Mutation: exakte Duplikate erkennen und durch frische Random-Bäume ersetzen.

### 1. Kanonische Arena-Form + Derived Equality

**Problem:** Derived `PartialEq` auf `Vec<Node>` vergleicht die Arena byteweise. Zwei strukturell identische Bäume können aber verschiedene Arena-Layouts haben, wenn sie unterschiedlich aufgebaut wurden. `random_tree()` baut in Einfügereihenfolge (nicht DFS-Preorder), also ist die Arena-Ordnung von der Bauhistorie abhängig.

**Lösung:** Kanonische Form erzwingen — alle Arenas in DFS-Preorder. Dann ist derived `Eq`/`Hash` korrekt.

```rust
// tree.rs — derive ergänzen
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cut { V, H }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Node {
    Leaf { photo_idx: u16, parent: Option<u16> },
    Internal { cut: Cut, left: u16, right: u16, parent: Option<u16> },
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SlicingTree { nodes: Vec<Node> }
```

```rust
/// Rebuilds the arena in DFS-Preorder (kanonische Form).
pub fn canonicalize(&self) -> SlicingTree { ... }
```

**Wo wird kanonische Form hergestellt?**

| Quelle | Arena-Form | Aktion |
| --- | --- | --- |
| `random_tree()` | Einfügereihenfolge | `canonicalize()` am Ende aufrufen |
| `rebuild_with_graft()` (Crossover) | Bereits DFS-Preorder | Nichts nötig |
| Mutation (Cut-Flip) | Unverändert | Nichts nötig |

Damit ist jeder Baum in der Population in kanonischer Form und `PartialEq`/`Hash` auf `Vec<Node>` korrekt.

### 2. Deduplizierung im Evolutionszyklus

```rust
fn deduplicate<R: Rng>(
    population: &mut Vec<LayoutIndividual>,
    context: &EvaluationContext,
    rng: &mut R,
) {
    let mut seen = HashSet::new();
    for individual in population.iter_mut() {
        if !seen.insert(individual.tree().clone()) {
            // Duplikat → durch frischen Random-Baum ersetzen
            let n = individual.tree().leaf_count();
            let tree = random_tree(n, rng);
            *individual = LayoutIndividual::from_tree(tree, context);
        }
    }
}
```

**Zeitpunkt:** Nach `build_next_population()` (Elite + Offspring zusammengeführt), vor dem Sortieren. So bleiben Eliten erhalten, aber Kopien davon werden durch frisches Material ersetzt.

### 3. Einordnung in den Evolutionszyklus

```rust
pub fn evolve(&mut self, ...) {
    let elite = self.population[..elite_count].to_vec();
    let selected = evolutor.select(&self.population);
    let mut offspring = evolutor.crossover(&selected);
    evolutor.mutate(&mut offspring);

    self.population = build_next_population(elite, offspring, target_size);
    deduplicate(&mut self.population, context, rng);  // NEU
    self.population.sort_by(|a, b| a.fitness().total_cmp(&b.fitness()));
}
```

## Warum nur exakte Duplikate?

- **Einfach**: Kein Schwellwert-Tuning, kein Distanzmaß
- **Sicher**: Entfernt nur echte Redundanz, keine nützlichen Varianten in der Nähe guter Lösungen
- **Billig**: `HashSet`-Insert ist O(1) amortisiert, Hash über ~3KB Arena

"Ähnliche" Bäume (z.B. gleiche Topologie, ein Cut anders) sind wertvolle Nachbarn im Suchraum und dürfen bleiben.

## Performance

- Hash pro Baum: O(N) — einmal über den Node-Vec
- Deduplizierung pro Generation: O(P * N) — P = Populationsgröße, N = Nodes pro Baum
- Bei P=100, N=9 (5 Fotos): ~900 Hash-Operationen → vernachlässigbar

## Implementation Checklist

- [ ] `canonicalize()` für `SlicingTree` implementieren (Arena in DFS-Preorder umbauen)
- [ ] `random_tree()`: `canonicalize()` am Ende aufrufen
- [ ] `PartialEq`, `Eq`, `Hash` für `Cut`, `Node`, `SlicingTree` ableiten
- [ ] `deduplicate()` implementieren
- [ ] In Evolutionszyklus einbauen (nach `build_next_population`, vor Sort)
- [ ] Test: Population nach Deduplizierung enthält keine identischen Bäume
- [ ] Test: Populationsgröße bleibt konstant

## Verwandte Dokumente

- [In-Page Ordering](in_page_ordering_improvement.md) — DFS-Indexing verkleinert den Suchraum, macht Diversity-Check wichtiger
- [Crossover Implementation](crossover_implementation.md) — Subtree-Swap erzeugt oft ähnliche Kinder

## Tests

```rust
#[test]
fn test_no_duplicates_after_deduplicate() {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    // Population mit absichtlichen Duplikaten erzeugen
    let tree = random_tree(5, &mut rng);
    let mut population = vec![
        LayoutIndividual::from_tree(tree.clone(), &ctx),
        LayoutIndividual::from_tree(tree.clone(), &ctx),
        LayoutIndividual::from_tree(tree.clone(), &ctx),
        LayoutIndividual::from_tree(random_tree(5, &mut rng), &ctx),
    ];

    deduplicate(&mut population, &ctx, &mut rng);

    let trees: HashSet<_> = population.iter().map(|i| i.tree().clone()).collect();
    assert_eq!(trees.len(), population.len()); // keine Duplikate
    assert_eq!(population.len(), 4); // Größe unverändert
}
```
