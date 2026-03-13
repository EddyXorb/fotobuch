# In-Page Ordering via DFS-Indexing

## Kernidee

Fotos werden deterministisch anhand der DFS-Preorder-Position im Slicing-Tree zugewiesen: Leaf 0 (DFS) bekommt das älteste Foto, Leaf 1 das zweitälteste, usw.

DFS-Preorder traversiert bei V-Cut links vor rechts, bei H-Cut oben vor unten. Dadurch gruppiert es räumlich zusammengehörige Fotos natürlich — innerhalb jeder Gruppe sind Fotos vertikal und horizontal korrekt geordnet. Das entspricht der Art, wie Menschen ein Fotobuch-Layout betrachten (gruppenweise, nicht strikt zeilenweise).

**Konsequenz:** `cost_reading_order` entfällt. Die Reihenfolge ist strukturell garantiert, nicht über Fitness-Penalties.

Der GA optimiert nur noch **Baumtopologie und Cut-Richtungen**. Die Fotozuweisung ergibt sich deterministisch aus dem Baum.

---

## Änderungen am GA

### 1. Fotozuweisung nach DFS-Order

Nach jeder Baum-Erzeugung/-Modifikation werden die Fotos per DFS-Traversal neu zugewiesen:

```rust
fn assign_photos_by_dfs(tree: &mut SlicingTree) {
    let mut counter: u16 = 0;
    // Braucht iterative DFS, da visit() nur &Node gibt
    assign_recursive(tree, 0, &mut counter);
}

fn assign_recursive(tree: &mut SlicingTree, idx: u16, counter: &mut u16) {
    match *tree.node(idx) {
        Node::Leaf { .. } => {
            if let Node::Leaf { ref mut photo_idx, .. } = tree.node_mut(idx) {
                *photo_idx = *counter;
                *counter += 1;
            }
        }
        Node::Internal { left, right, .. } => {
            assign_recursive(tree, left, counter);
            assign_recursive(tree, right, counter);
        }
    }
}
```

Aufrufe:
- `random_tree()`: nach Baumerzeugung statt Fisher-Yates-Shuffle
- Crossover: nach Subtree-Swap auf beiden Kindern

**Nicht nötig nach Cut-Flip:** DFS traversiert immer `left` dann `right`. Ein Cut-Flip ändert nur V↔H (räumliche Anordnung), nicht welcher Node `left`/`right` ist. Die DFS-Leaf-Reihenfolge bleibt identisch.

### 2. Mutation: Cut-Flip statt Leaf-Swap

Aktuell tauscht `mutate()` nur `photo_idx` zweier Leaves — das widerspricht der deterministischen Zuweisung. Stattdessen:

```rust
pub(crate) fn mutate<R: Rng>(tree: &mut SlicingTree, rng: &mut R) {
    let internal_indices: Vec<u16> = tree.nodes().iter().enumerate()
        .filter_map(|(i, n)| if n.is_internal() { Some(i as u16) } else { None })
        .collect();

    if internal_indices.is_empty() {
        return;
    }

    // Flip eines zufälligen Cuts
    let idx = internal_indices[rng.gen_range(0..internal_indices.len())];
    if let Node::Internal { ref mut cut, .. } = tree.node_mut(idx) {
        *cut = match cut {
            Cut::V => Cut::H,
            Cut::H => Cut::V,
        };
    }

    // Kein assign_photos_by_dfs() nötig — Cut-Flip ändert die DFS-Reihenfolge nicht
}
```

Ein Cut-Flip ändert die räumliche Anordnung eines Subtrees drastisch (nebeneinander ↔ übereinander), aber **nicht** die DFS-Leaf-Reihenfolge. Die Fotozuweisung bleibt stabil.

### 3. Crossover: Reassign nach Subtree-Swap

Der bestehende Crossover tauscht Subtree-Topologien zwischen zwei Bäumen. Danach müssen die Fotos in beiden Kindern per DFS neu zugewiesen werden:

```rust
// In crossover(), nach dem Subtree-Swap:
assign_photos_by_dfs(&mut child_a);
assign_photos_by_dfs(&mut child_b);
```

### 4. `cost_reading_order` entfernen

- `cost_reading_order()` aus `fitness.rs` entfernen
- `w_order` aus `FitnessWeights` entfernen
- `order`-Feld aus `CostBreakdown` entfernen
- Entsprechende CLI-Parameter und Serde-Felder bereinigen

---

## Konfiguration: `enforce_order`

Das DFS-Ordering kann per Config deaktiviert werden. Dann verhält sich der GA wie bisher (zufällige Fotozuweisung, Leaf-Swap-Mutation, kein `cost_reading_order`).

```rust
// In der Solver-Config (z.B. PageLayoutSolverConfig oder GaConfig)
#[serde(default = "default_true")]
pub enforce_order: bool,
```

**Auswirkung auf den GA:**

| Komponente | `enforce_order: true` | `enforce_order: false` |
|---|---|---|
| `random_tree()` | `assign_photos_by_dfs()` | Fisher-Yates-Shuffle (wie bisher) |
| Mutation | Cut-Flip (kein Reassign nötig) | Leaf-Swap (wie bisher) |
| Crossover | Subtree-Swap + Reassign | Subtree-Swap (wie bisher) |

Die Entscheidung wird einmal beim Start des Solvers getroffen und als Parameter an `mutate()` und `crossover()` durchgereicht — kein Runtime-Overhead.

---

## Warum das funktioniert

1. **Strukturelle Garantie**: Baumtopologie → DFS-Leaf-Reihenfolge → Fotozuweisung. Kein Individuum kann eine falsche Reihenfolge haben.

2. **GA bleibt mächtig**: Cut-Flips ändern das Layout drastisch. Crossover erzeugt neue Topologien. Beide erhalten die Reihenfolge automatisch.

3. **Performance-Gewinn**: `cost_reading_order` entfällt (O(N log N) Sortierung pro Fitness-Evaluation). `assign_photos_by_dfs` ist O(N) und wird nur bei Baumerzeugung und Crossover aufgerufen — bei Mutation (Cut-Flip) ist es nicht nötig.

4. **Einfacherer Code**: Kein Tuning von `w_order` nötig. Keine Konflikte zwischen Ordering-Cost und anderen Fitness-Komponenten.

---

## Implementation Checklist

- [ ] `assign_photos_by_dfs()` in `tree.rs` implementieren
- [ ] `random_tree()`: Fisher-Yates-Shuffle durch `assign_photos_by_dfs()` ersetzen
- [ ] `mutate()`: Leaf-Swap durch Cut-Flip ersetzen (kein Reassign nötig)
- [ ] Crossover: `assign_photos_by_dfs()` auf Kinder aufrufen
- [ ] `cost_reading_order`, `w_order`, `CostBreakdown::order` entfernen
- [ ] `enforce_order: bool` in Solver-Config aufnehmen (Default: `true`)
- [ ] `mutate()` und `crossover()` verzweigen je nach `enforce_order`
- [ ] CLI/Config-Parameter bereinigen
- [ ] Tests anpassen

## Tests

```rust
#[test]
fn test_dfs_assigns_sequential_indices() {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let tree = random_tree(5, &mut rng);

    // DFS-Traversal muss 0, 1, 2, 3, 4 in Leaf-Reihenfolge ergeben
    let mut photos = Vec::new();
    tree.visit(|_, node| {
        if let Node::Leaf { photo_idx, .. } = node {
            photos.push(*photo_idx);
        }
    });
    assert_eq!(photos, vec![0, 1, 2, 3, 4]);
}

#[test]
fn test_cut_flip_preserves_ordering_invariant() {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let mut tree = random_tree(5, &mut rng);

    for _ in 0..100 {
        mutate(&mut tree, &mut rng);

        let mut photos = Vec::new();
        tree.visit(|_, node| {
            if let Node::Leaf { photo_idx, .. } = node {
                photos.push(*photo_idx);
            }
        });
        assert_eq!(photos, vec![0, 1, 2, 3, 4]);
    }
}

#[test]
fn test_crossover_preserves_ordering() {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let tree_a = random_tree(5, &mut rng);
    let tree_b = random_tree(5, &mut rng);

    if let Some((child_a, child_b)) = crossover(&tree_a, &tree_b, &mut rng) {
        // Beide Kinder müssen die Invariante erfüllen
        for child in [&child_a, &child_b] {
            let mut photos = Vec::new();
            child.visit(|_, node| {
                if let Node::Leaf { photo_idx, .. } = node {
                    photos.push(*photo_idx);
                }
            });
            assert_eq!(photos, vec![0, 1, 2, 3, 4]);
        }
    }
}
```

## Verwandte Dokumente

- [Population Diversity](population_diversity.md) — Duplikat-Eliminierung (#21), besonders relevant da `enforce_order` den Suchraum verkleinert
- [Crossover Implementation](crossover_implementation.md) — Subtree-Swap, muss um `assign_photos_by_dfs()` ergänzt werden

## Edge Cases

- **1 Foto pro Seite**: Trivial — ein Leaf, kein Cut-Flip möglich
- **2 Fotos**: Ein Internal-Node, Cut-Flip wechselt zwischen nebeneinander/übereinander
- **Gleiche Aspect-Ratios**: Cut-Flips sind hier besonders wirkungsvoll, da V↔H den visuellen Eindruck stark ändert
