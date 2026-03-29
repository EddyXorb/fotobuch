# In-Page Ordering via DFS-Indexing

## Kernidee

Fotos werden deterministisch anhand der DFS-Preorder-Position im Slicing-Tree zugewiesen: Leaf 0 (DFS) bekommt das älteste Foto, Leaf 1 das zweitälteste, usw.

DFS-Preorder traversiert bei V-Cut links vor rechts, bei H-Cut oben vor unten. Dadurch entspricht die Foto-Reihenfolge der natürlichen Leserichtung — ohne Fitness-Penalty.

**Konsequenz:** `cost_reading_order` entfällt. Die Reihenfolge ist strukturell garantiert.

Der GA optimiert nur noch **Baumtopologie und Cut-Richtungen**. Die Fotozuweisung ergibt sich deterministisch aus dem Baum.

## Änderungen am GA

### Fotozuweisung

`assign_photos_by_dfs()` in `tree/create.rs`: DFS-Traversal, Leaf-Indices sequenziell 0..N vergeben.

Wird aufgerufen nach:
- `random_tree()` (statt Fisher-Yates-Shuffle)
- Crossover (nach Subtree-Swap auf beiden Kindern)

**Nicht nötig nach Cut-Flip:** DFS traversiert immer `left` dann `right`. Ein Cut-Flip ändert nur V↔H (räumliche Anordnung), nicht die `left`/`right`-Zuordnung. Die DFS-Leaf-Reihenfolge bleibt identisch.

### Mutation: Cut-Flip statt Leaf-Swap

Ein Cut-Flip ändert die räumliche Anordnung eines Subtrees drastisch (nebeneinander ↔ übereinander), aber nicht die DFS-Leaf-Reihenfolge. Kein Reassign nötig.

## Konfiguration: `enforce_order`

In `GaConfig` steuerbar (Default: `true`). Bei `false`: Fisher-Yates-Shuffle, Leaf-Swap-Mutation, kein Reassign nach Crossover (Legacy-Verhalten).

| Komponente      | `enforce_order: true`    | `enforce_order: false` |
| --------------- | ------------------------ | ---------------------- |
| `random_tree()` | `assign_photos_by_dfs()` | Fisher-Yates-Shuffle   |
| Mutation        | Cut-Flip                 | Leaf-Swap              |
| Crossover       | Subtree-Swap + Reassign  | Subtree-Swap           |

## Warum das funktioniert

1. **Strukturelle Garantie**: Topologie → DFS-Leaf-Reihenfolge → Fotozuweisung. Kein Individuum kann falsche Reihenfolge haben.
2. **GA bleibt mächtig**: Cut-Flips ändern Layout drastisch. Crossover erzeugt neue Topologien.
3. **Performance**: `assign_photos_by_dfs` ist O(N), nur bei Baumerzeugung und Crossover nötig.
