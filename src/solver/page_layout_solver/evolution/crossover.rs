//! Crossover operator for slicing trees.

use super::super::individual::LayoutIndividual;
use super::super::tree::create::assign_photos_by_dfs;
use super::EvaluationContext;
use crate::solver::page_layout_solver::tree::{Cut, Node, SlicingTree};
use rand::Rng;

/// Applies crossover to parents with given rate.
pub(super) fn apply_crossover<R: Rng>(
    parents: &[LayoutIndividual],
    crossover_rate: f64,
    context: &EvaluationContext,
    rng: &mut R,
) -> Vec<LayoutIndividual> {
    let mut offspring = Vec::with_capacity(parents.len());

    for chunk in parents.chunks_exact(2) {
        if rng.r#gen::<f64>() < crossover_rate {
            crossover_pair(chunk, context, rng, &mut offspring);
        } else {
            offspring.extend_from_slice(chunk);
        }
    }

    // Handle odd parent
    if parents.len() % 2 == 1 {
        offspring.push(parents.last().unwrap().clone());
    }

    offspring
}

/// Performs crossover on a pair of parents.
fn crossover_pair<R: Rng>(
    pair: &[LayoutIndividual],
    context: &EvaluationContext,
    rng: &mut R,
    offspring: &mut Vec<LayoutIndividual>,
) {
    let tree_a = pair[0].tree();
    let tree_b = pair[1].tree();

    if let Some((child_a, child_b)) = crossover(tree_a, tree_b, rng, context.enforce_order) {
        offspring.push(LayoutIndividual::from_tree(child_a, context));
        offspring.push(LayoutIndividual::from_tree(child_b, context));
    } else {
        offspring.extend_from_slice(pair);
    }
}

/// Performs crossover between two slicing trees.
///
/// Exchanges subtrees with equal leaf counts (≥3) between two parent trees.
/// If `enforce_order` is true, reassigns photos by DFS after the swap.
/// Otherwise, photo labels remain in their original positions.
///
/// Returns None if no compatible subtrees exist.
pub(crate) fn crossover<R: Rng>(
    tree_a: &SlicingTree,
    tree_b: &SlicingTree,
    rng: &mut R,
    enforce_order: bool,
) -> Option<(SlicingTree, SlicingTree)> {
    // Step 1: Compute leaf counts for both trees
    let counts_a = leaf_counts(tree_a);
    let counts_b = leaf_counts(tree_b);

    // Step 2: Find compatible pairs
    let pairs = find_compatible_pairs(tree_a, tree_b, &counts_a, &counts_b);
    if pairs.is_empty() {
        return None;
    }

    // Pick a random compatible pair
    let &(node_a, node_b) = pairs.get(rng.gen_range(0..pairs.len()))?;

    // Step 3: Extract subtree topologies
    let (topo_a, labels_a) = extract_subtree(tree_a, node_a);
    let (topo_b, labels_b) = extract_subtree(tree_b, node_b);

    // Step 4 & 5: Rebuild trees with swapped topologies
    let mut new_a = rebuild_with_graft(tree_a, node_a, &topo_b, &labels_a);
    let mut new_b = rebuild_with_graft(tree_b, node_b, &topo_a, &labels_b);

    // Step 6: If enforce_order, reassign photos by DFS
    if enforce_order {
        assign_photos_by_dfs(&mut new_a);
        assign_photos_by_dfs(&mut new_b);
    }

    Some((new_a, new_b))
}

/// Computes the number of leaves in each node's subtree.
fn leaf_counts(tree: &SlicingTree) -> Vec<u16> {
    let mut counts = vec![0u16; tree.len()];

    fn walk(nodes: &[Node], idx: u16, counts: &mut [u16]) -> u16 {
        match &nodes[idx as usize] {
            Node::Leaf { .. } => {
                counts[idx as usize] = 1;
                1
            }
            Node::Internal { left, right, .. } => {
                let c = walk(nodes, *left, counts) + walk(nodes, *right, counts);
                counts[idx as usize] = c;
                c
            }
        }
    }

    walk(tree.nodes(), 0, &mut counts);
    counts
}

/// Finds all compatible (node_a, node_b) pairs for crossover.
///
/// Two nodes are compatible if:
/// - Both are internal nodes (not leaves)
/// - They have the same number of leaves (≥3)
/// - Neither is the root node (index 0)
fn find_compatible_pairs(
    tree_a: &SlicingTree,
    tree_b: &SlicingTree,
    counts_a: &[u16],
    counts_b: &[u16],
) -> Vec<(u16, u16)> {
    use std::collections::HashMap;

    // Group internal nodes from B by leaf count
    let mut b_by_count: HashMap<u16, Vec<u16>> = HashMap::new();
    for (idx, node) in tree_b.nodes().iter().enumerate() {
        if matches!(node, Node::Internal { .. }) && idx != 0 {
            let count = counts_b[idx];
            if count >= 3 {
                b_by_count.entry(count).or_default().push(idx as u16);
            }
        }
    }

    // Find matching nodes in A
    let mut pairs = Vec::new();
    for (idx, node) in tree_a.nodes().iter().enumerate() {
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

/// A node in the topology tree - structure only, no photo labels.
#[derive(Clone, Debug)]
enum TopoNode {
    Leaf,
    Internal { cut: Cut },
}

/// Extracts the topology (structure) of a subtree in pre-order.
/// Returns the topology and the leaf labels in pre-order.
fn extract_subtree(tree: &SlicingTree, root_idx: u16) -> (Vec<TopoNode>, Vec<u16>) {
    let mut topo = Vec::new();
    let mut labels = Vec::new();

    fn walk(nodes: &[Node], idx: u16, topo: &mut Vec<TopoNode>, labels: &mut Vec<u16>) {
        match &nodes[idx as usize] {
            Node::Leaf { photo_idx, .. } => {
                topo.push(TopoNode::Leaf);
                labels.push(*photo_idx);
            }
            Node::Internal {
                cut, left, right, ..
            } => {
                topo.push(TopoNode::Internal { cut: *cut });
                walk(nodes, *left, topo, labels);
                walk(nodes, *right, topo, labels);
            }
        }
    }

    walk(tree.nodes(), root_idx, &mut topo, &mut labels);
    (topo, labels)
}

/// Rebuilds a tree with the subtree at target_idx replaced by new_topo.
/// Labels from the original subtree are applied to the new topology in pre-order.
fn rebuild_with_graft(
    tree: &SlicingTree,
    target_idx: u16,
    new_topo: &[TopoNode],
    labels: &[u16],
) -> SlicingTree {
    let mut new_nodes: Vec<Node> = Vec::with_capacity(tree.len());
    let mut label_iter = labels.iter().copied();

    /// Recursively copy or graft nodes. Returns the new index.
    #[allow(clippy::too_many_arguments)]
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
            // Replace this subtree
            graft_topo(new_topo, topo_cursor, label_iter, new_nodes, parent);
            return my_idx;
        }

        // Copy original node
        match &old.nodes()[old_idx as usize] {
            Node::Leaf { photo_idx, .. } => {
                new_nodes.push(Node::Leaf {
                    photo_idx: *photo_idx,
                    parent,
                });
            }
            Node::Internal {
                cut, left, right, ..
            } => {
                // Push placeholder
                new_nodes.push(Node::Internal {
                    cut: *cut,
                    left: 0,
                    right: 0,
                    parent,
                });

                let new_left = copy_or_graft(
                    old,
                    *left,
                    target_idx,
                    new_topo,
                    topo_cursor,
                    label_iter,
                    new_nodes,
                    Some(my_idx),
                );
                let new_right = copy_or_graft(
                    old,
                    *right,
                    target_idx,
                    new_topo,
                    topo_cursor,
                    label_iter,
                    new_nodes,
                    Some(my_idx),
                );

                // Update left/right in the placeholder
                if let Node::Internal {
                    left: l, right: r, ..
                } = &mut new_nodes[my_idx as usize]
                {
                    *l = new_left;
                    *r = new_right;
                }
            }
        }

        my_idx
    }

    /// Grafts the new topology and assigns labels.
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
                let photo_idx = labels.next().expect("label iterator exhausted");
                new_nodes.push(Node::Leaf { photo_idx, parent });
            }
            TopoNode::Internal { cut } => {
                new_nodes.push(Node::Internal {
                    cut: *cut,
                    left: 0,
                    right: 0,
                    parent,
                });

                let new_left = graft_topo(topo, cursor, labels, new_nodes, Some(my_idx));
                let new_right = graft_topo(topo, cursor, labels, new_nodes, Some(my_idx));

                if let Node::Internal {
                    left: l, right: r, ..
                } = &mut new_nodes[my_idx as usize]
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
        tree,
        0,
        target_idx,
        new_topo,
        &mut topo_cursor,
        &mut label_iter,
        &mut new_nodes,
        None,
    );

    SlicingTree::new(new_nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::page_layout_solver::tree::create::random_tree;
    use crate::solver::page_layout_solver::tree::validate::validate_tree;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn test_crossover_basic() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let tree_a = random_tree(5, &mut rng, true);
        let tree_b = random_tree(5, &mut rng, true);

        let result = crossover(&tree_a, &tree_b, &mut rng, true);

        if let Some((new_a, new_b)) = result {
            // Both results should be valid trees
            assert!(validate_tree(&new_a).is_ok());
            assert!(validate_tree(&new_b).is_ok());

            // Same number of leaves as parents
            assert_eq!(new_a.leaf_count(), tree_a.leaf_count());
            assert_eq!(new_b.leaf_count(), tree_b.leaf_count());
        }
        // If None, crossover wasn't possible (no compatible subtrees >= 3 leaves)
    }

    #[test]
    fn test_crossover_preserves_photos() {
        let mut rng = ChaCha8Rng::seed_from_u64(123);

        // Try multiple times to get a successful crossover
        for _ in 0..50 {
            let tree_a = random_tree(10, &mut rng, true);
            let tree_b = random_tree(10, &mut rng, true);

            if let Some((new_a, new_b)) = crossover(&tree_a, &tree_b, &mut rng, true) {
                // Collect photo indices from original trees
                let mut photos_a: Vec<u16> = Vec::new();
                let mut photos_b: Vec<u16> = Vec::new();

                for node in tree_a.nodes() {
                    if let Node::Leaf { photo_idx, .. } = node {
                        photos_a.push(*photo_idx);
                    }
                }
                for node in tree_b.nodes() {
                    if let Node::Leaf { photo_idx, .. } = node {
                        photos_b.push(*photo_idx);
                    }
                }

                // Collect photo indices from new trees
                let mut new_photos_a: Vec<u16> = Vec::new();
                let mut new_photos_b: Vec<u16> = Vec::new();

                for node in new_a.nodes() {
                    if let Node::Leaf { photo_idx, .. } = node {
                        new_photos_a.push(*photo_idx);
                    }
                }
                for node in new_b.nodes() {
                    if let Node::Leaf { photo_idx, .. } = node {
                        new_photos_b.push(*photo_idx);
                    }
                }

                // Photos should be permutations of original
                photos_a.sort_unstable();
                photos_b.sort_unstable();
                new_photos_a.sort_unstable();
                new_photos_b.sort_unstable();

                assert_eq!(photos_a, new_photos_a);
                assert_eq!(photos_b, new_photos_b);

                return; // Test passed
            }
        }
    }

    #[test]
    fn test_crossover_multiple_attempts() {
        let mut rng = ChaCha8Rng::seed_from_u64(999);

        // With enough photos and attempts, crossover should succeed
        let mut success_count = 0;
        for _ in 0..100 {
            let tree_a = random_tree(10, &mut rng, true);
            let tree_b = random_tree(10, &mut rng, true);

            if let Some((new_a, new_b)) = crossover(&tree_a, &tree_b, &mut rng, true) {
                assert!(validate_tree(&new_a).is_ok());
                assert!(validate_tree(&new_b).is_ok());
                success_count += 1;
            }
        }

        // Should have at least some successful crossovers
        assert!(
            success_count > 0,
            "Crossover should succeed at least sometimes with 10 photos"
        );
    }

    #[test]
    fn test_crossover_small_trees() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        // With only 2 photos, no subtree has >= 3 leaves
        let tree_a = random_tree(2, &mut rng, true);
        let tree_b = random_tree(2, &mut rng, true);
        assert!(crossover(&tree_a, &tree_b, &mut rng, true).is_none());

        // With 3 photos, might work if structure is right
        let tree_a = random_tree(3, &mut rng, true);
        let tree_b = random_tree(3, &mut rng, true);
        let _ = crossover(&tree_a, &tree_b, &mut rng, true);
        // Don't assert result - depends on random structure
    }

    #[test]
    fn test_crossover_preserves_ordering() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let tree_a = random_tree(5, &mut rng, true);
        let tree_b = random_tree(5, &mut rng, true);

        if let Some((child_a, child_b)) = crossover(&tree_a, &tree_b, &mut rng, true) {
            // Both children must maintain the ordering invariant
            for child in [&child_a, &child_b] {
                let mut photos = Vec::new();
                child.visit(|_, node| {
                    if let Node::Leaf { photo_idx, .. } = node {
                        photos.push(*photo_idx);
                    }
                });
                assert_eq!(
                    photos,
                    vec![0, 1, 2, 3, 4],
                    "Ordering invariant violated in crossover child"
                );
            }
        }
    }
}
