# Crossover: Implementierungsdetail

Beschreibt die Crossover-Operation für Arena-basierte Slicing-Trees im Detail.

## Überblick

Crossover tauscht Teilbäume zwischen zwei Eltern-Bäumen. Die Besonderheit: **Blatt-Labels (photo_idx) bleiben im Original-Baum.** Nur die Topologie (Baumstruktur + Schnittrichtungen) wird getauscht.

```
Eltern:   Baum A, Baum B  (gleiche Fotos, unterschiedliche Struktur)
Ergebnis: Baum A', Baum B' (neue Strukturen, gleiche Foto-Zuordnungen)
```

## Algorithmus in 5 Schritten

### Schritt 1: Blattanzahl pro Knoten berechnen

Ein bottom-up Pass über beide Bäume. Jeder Knoten bekommt die Anzahl Blätter in seinem Teilbaum.

```rust
/// Berechnet die Blattanzahl für jeden Knoten im Baum.
fn leaf_counts(tree: &SlicingTree) -> Vec<u16> {
    let mut counts = vec![0u16; tree.nodes.len()];

    fn walk(nodes: &[Node], idx: u16, counts: &mut [u16]) -> u16 {
        match nodes[idx as usize] {
            Node::Leaf { .. } => {
                counts[idx as usize] = 1;
                1
            }
            Node::Internal { left, right, .. } => {
                let c = walk(nodes, left, counts) + walk(nodes, right, counts);
                counts[idx as usize] = c;
                c
            }
        }
    }

    walk(&tree.nodes, 0, &mut counts);
    counts
}
```

Ergebnis: `Vec<u16>`, Index = Node-Index im Arena-Vec.

### Schritt 2: Kompatible Paare finden

Zwei Teilbäume sind kompatibel wenn:
- Beide **innere Knoten** sind (nicht Blätter)
- Gleiche Blattanzahl ≥ 3
- Keiner der beiden ist Root seines Baums (sonst tauscht man den ganzen Baum)

```rust
/// Findet alle kompatiblen (node_a, node_b)-Paare für Crossover.
fn find_compatible_pairs(
    tree_a: &SlicingTree,
    tree_b: &SlicingTree,
    counts_a: &[u16],
    counts_b: &[u16],
) -> Vec<(u16, u16)> {
    // Innere Knoten aus B, gruppiert nach Blattanzahl
    let mut b_by_count: HashMap<u16, Vec<u16>> = HashMap::new();
    for (idx, node) in tree_b.nodes.iter().enumerate() {
        if matches!(node, Node::Internal { .. }) && idx != 0 {
            let count = counts_b[idx];
            if count >= 3 {
                b_by_count.entry(count).or_default().push(idx as u16);
            }
        }
    }

    let mut pairs = Vec::new();
    for (idx, node) in tree_a.nodes.iter().enumerate() {
        if matches!(node, Node::Internal { .. }) && idx != 0 {
            let count = counts_a[idx];
            if let Some(b_nodes) = b_by_count.get(&count) {
                for &b_idx in b_nodes {
                    pairs.push((idx as u16, b_idx));
                }
            }
        }
    }

    pairs
}
```

Falls `pairs` leer: Crossover nicht möglich, `None` zurückgeben.

### Schritt 3: Teilbaum-Topologie extrahieren

Die Topologie ist die Baumstruktur ohne Blatt-Labels — nur Schnittrichtungen und Verzweigungen.

Darstellung als kompakter Vec in Pre-Order:

```rust
/// Ein Knoten der Topologie — nur Struktur, keine Foto-Zuordnung.
#[derive(Clone)]
enum TopoNode {
    Leaf,
    Internal { cut: Cut },
}

/// Extrahiert die Topologie eines Teilbaums in Pre-Order.
/// Sammelt gleichzeitig die Blatt-Labels in Reihenfolge.
fn extract_subtree(
    tree: &SlicingTree,
    root_idx: u16,
) -> (Vec<TopoNode>, Vec<u16>) {
    let mut topo = Vec::new();
    let mut labels = Vec::new();

    fn walk(
        nodes: &[Node],
        idx: u16,
        topo: &mut Vec<TopoNode>,
        labels: &mut Vec<u16>,
    ) {
        match nodes[idx as usize] {
            Node::Leaf { photo_idx, .. } => {
                topo.push(TopoNode::Leaf);
                labels.push(photo_idx);
            }
            Node::Internal { cut, left, right, .. } => {
                topo.push(TopoNode::Internal { cut });
                walk(nodes, left, topo, labels);
                walk(nodes, right, topo, labels);
            }
        }
    }

    walk(&tree.nodes, root_idx, &mut topo, &mut labels);
    (topo, labels)
}
```

`topo` enthält die Struktur, `labels` die `photo_idx`-Werte der Blätter in Pre-Order-Reihenfolge.

### Schritt 4: Neuen Teilbaum einsetzen

Der Kern der Operation: Ersetze den Teilbaum an Position `target_idx` im Zielbaum durch eine neue Topologie und weise die **Original-Labels** des Zielbaums zu.

Die einfachste korrekte Strategie: **Den gesamten Baum neu aufbauen.** Der Teilbaum an `target_idx` wird durch die neue Topologie ersetzt, alle anderen Knoten bleiben gleich.

```rust
/// Baut einen neuen Baum, in dem der Teilbaum an `target_idx`
/// durch `new_topo` ersetzt wird. Labels kommen aus `labels`.
fn rebuild_with_graft(
    tree: &SlicingTree,
    target_idx: u16,
    new_topo: &[TopoNode],
    labels: &[u16],
) -> SlicingTree {
    let mut new_nodes: Vec<Node> = Vec::with_capacity(tree.nodes.len());
    let mut label_iter = labels.iter().copied();

    /// Rekursiver Aufbau. Gibt den Index des eingefügten Knotens zurück.
    fn copy_or_graft(
        old: &SlicingTree,
        old_idx: u16,
        target_idx: u16,
        new_topo: &[TopoNode],
        topo_cursor: &mut usize,
        label_iter: &mut impl Iterator<Item = u16>,
        new_nodes: &mut Vec<Node>,
        parent: Option<u16>,
    ) -> u16 {
        let my_idx = new_nodes.len() as u16;

        if old_idx == target_idx {
            // --- Teilbaum ersetzen ---
            graft_topo(new_topo, topo_cursor, label_iter, new_nodes, parent);
            return my_idx;
        }

        // --- Original-Knoten kopieren ---
        match old.nodes[old_idx as usize] {
            Node::Leaf { photo_idx, .. } => {
                new_nodes.push(Node::Leaf { photo_idx, parent });
            }
            Node::Internal { cut, left, right, .. } => {
                // Platzhalter — left/right werden nachgetragen
                new_nodes.push(Node::Internal {
                    cut,
                    left: 0,
                    right: 0,
                    parent,
                });

                let new_left = copy_or_graft(
                    old, left, target_idx, new_topo,
                    topo_cursor, label_iter, new_nodes, Some(my_idx),
                );
                let new_right = copy_or_graft(
                    old, right, target_idx, new_topo,
                    topo_cursor, label_iter, new_nodes, Some(my_idx),
                );

                // left/right nachtragen
                if let Node::Internal { left: l, right: r, .. }
                    = &mut new_nodes[my_idx as usize]
                {
                    *l = new_left;
                    *r = new_right;
                }
            }
        }

        my_idx
    }

    /// Setzt die neue Topologie ein und weist Labels zu.
    fn graft_topo(
        topo: &[TopoNode],
        cursor: &mut usize,
        labels: &mut impl Iterator<Item = u16>,
        new_nodes: &mut Vec<Node>,
        parent: Option<u16>,
    ) -> u16 {
        let my_idx = new_nodes.len() as u16;
        let node = &topo[*cursor];
        *cursor += 1;

        match node {
            TopoNode::Leaf => {
                let photo_idx = labels.next().expect("label exhausted");
                new_nodes.push(Node::Leaf { photo_idx, parent });
            }
            TopoNode::Internal { cut } => {
                new_nodes.push(Node::Internal {
                    cut: *cut,
                    left: 0,
                    right: 0,
                    parent,
                });

                let new_left = graft_topo(
                    topo, cursor, labels, new_nodes, Some(my_idx),
                );
                let new_right = graft_topo(
                    topo, cursor, labels, new_nodes, Some(my_idx),
                );

                if let Node::Internal { left: l, right: r, .. }
                    = &mut new_nodes[my_idx as usize]
                {
                    *l = new_left;
                    *r = new_right;
                }
            }
        }

        my_idx
    }

    let mut topo_cursor = 0;
    copy_or_graft(
        tree, 0, target_idx, new_topo,
        &mut topo_cursor, &mut label_iter,
        &mut new_nodes, None,
    );

    SlicingTree { nodes: new_nodes }
}
```

**Warum Neuaufbau statt In-Place-Modifikation:** Der neue Teilbaum kann eine andere Anzahl innerer Knoten haben als der alte (bei gleicher Blattanzahl, aber anderer Struktur). In-Place würde Verschieben und Re-Indexierung aller Referenzen erfordern — fehleranfällig und nicht schneller bei <200 Knoten.

**Nebeneffekt:** Die Knoten liegen nach dem Rebuild in Pre-Order im Vec. Das ist sogar vorteilhaft für Cache-Lokalität bei der Traversierung.

### Schritt 5: Zusammensetzen

```rust
/// Crossover zwischen zwei Bäumen.
/// Gibt None zurück falls keine kompatiblen Teilbäume existieren.
fn crossover(
    tree_a: &SlicingTree,
    tree_b: &SlicingTree,
    rng: &mut impl Rng,
) -> Option<(SlicingTree, SlicingTree)> {
    let counts_a = leaf_counts(tree_a);
    let counts_b = leaf_counts(tree_b);

    let pairs = find_compatible_pairs(tree_a, tree_b, &counts_a, &counts_b);
    if pairs.is_empty() {
        return None;
    }

    let &(node_a, node_b) = pairs.choose(rng).unwrap();

    // Topologien extrahieren
    let (topo_a, labels_a) = extract_subtree(tree_a, node_a);
    let (topo_b, labels_b) = extract_subtree(tree_b, node_b);

    // Topologien tauschen, Labels behalten
    let new_a = rebuild_with_graft(tree_a, node_a, &topo_b, &labels_a);
    let new_b = rebuild_with_graft(tree_b, node_b, &topo_a, &labels_b);

    Some((new_a, new_b))
}
```

## Beispiel

Zwei Bäume mit 5 Fotos (P0..P4):

```
Baum A:                    Baum B:
       H                          V
      / \                        / \
     V    P4                    H    V
    / \                        / \  / \
   H   P2                   P0  P1 P3 P4
  / \
 P0  P1

Teilbaum A an V (3 Blätter: P0,P1,P2)
Teilbaum B an H (2 Blätter: P0,P1) — nicht kompatibel (< 3)
Teilbaum B an V (2 Blätter: P3,P4) — nicht kompatibel (≠ 3)
→ Kein Crossover möglich in diesem Fall.
```

Anderes Beispiel wo es klappt (beide haben einen 3-Blatt-Teilbaum):

```
Baum A:                    Baum B:
       H                          H
      / \                        / \
    [V]    P4                   P0  [V]
    / \                             / \
   H   P2                         H   P4
  / \                             / \
 P0  P1                         P1  P2

Teilbaum A bei [V]: Topo = [V, H, Leaf, Leaf, Leaf], Labels = [P0, P1, P2]
Teilbaum B bei [V]: Topo = [V, H, Leaf, Leaf, Leaf], Labels = [P1, P2, P4]
```

Nach Crossover (Topologien tauschen, Labels behalten):

```
Baum A': Topologie von B's [V], Labels von A = [P0, P1, P2]
       H
      / \
    [V]    P4
    / \
   H   P2          ← Labels [P0, P1, P2] auf neue Topo-Blätter verteilt
  / \
 P0  P1

Baum B': Topologie von A's [V], Labels von B = [P1, P2, P4]
       H
      / \
     P0  [V]
         / \
        H   P4      ← Labels [P1, P2, P4] auf neue Topo-Blätter verteilt
       / \
      P1  P2
```

In diesem Fall haben beide Teilbäume zufällig die gleiche Topologie, daher sieht die Struktur gleich aus — nur die Labels unterscheiden sich. Bei unterschiedlichen Topologien ändert sich die Baumform.

## Invarianten nach Crossover

Beide Ergebnis-Bäume müssen folgendes erfüllen (via `validate_tree()`):

1. Genau N Blätter, N−1 innere Knoten
2. Jedes `photo_idx` kommt genau einmal vor (Permutation von 0..N)
3. Alle `left`/`right`-Indizes gültig
4. Alle `parent`-Referenzen konsistent, Root hat `parent: None`
5. Blattanzahl unverändert gegenüber Elternbaum

Punkt 1 und 2 sind garantiert durch die Konstruktion: gleiche Blattanzahl der getauschten Teilbäume + Labels bleiben im Original-Baum. Trotzdem in Tests immer `validate_tree()` aufrufen.

## Performance-Überlegungen

| Operation | Komplexität |
|---|---|
| `leaf_counts()` | O(N) |
| `find_compatible_pairs()` | O(N²) worst case, O(N) typical |
| `extract_subtree()` | O(k), k = Blätter im Teilbaum |
| `rebuild_with_graft()` | O(N) |
| **Gesamt** | **O(N²)** durch Pair-Finding |

Bei N≤100 ist O(N²) = 10.000 Operationen — irrelevant. Falls es doch zum Bottleneck wird: `find_compatible_pairs()` kann durch Gruppierung nach Blattanzahl auf O(N) amortisiert werden (HashMap ist bereits so implementiert).

## Offene Entscheidung: Label-Reihenfolge

Die Labels werden in **Pre-Order-Reihenfolge** aus dem alten Teilbaum gelesen und in derselben Reihenfolge auf die Blätter der neuen Topologie verteilt. Das heißt:

- Das "erste" Blatt (Pre-Order) im alten Teilbaum bekommt dasselbe Label wie das "erste" Blatt im neuen Teilbaum
- Die Zuordnung ist deterministisch, nicht zufällig

Alternative: Labels **zufällig** auf die neuen Blätter verteilen. Das erhöht die Diversität, kann aber gute lokale Foto-Nachbarschaften zerstören. Beides ist korrekt — kann bei Bedarf als Parameter steuerbar gemacht werden.
