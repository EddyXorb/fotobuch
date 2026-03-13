//! Mutation operator for slicing trees.

use super::super::individual::LayoutIndividual;
use super::EvaluationContext;
use crate::solver::page_layout_solver::tree::{Cut, Node, SlicingTree};
use rand::Rng;

/// Applies mutation to individuals with given rate.
pub(super) fn apply_mutation<R: Rng>(
    individuals: &mut [LayoutIndividual],
    mutation_rate: f64,
    context: &EvaluationContext,
    rng: &mut R,
    enforce_order: bool,
) {
    for individual in individuals.iter_mut() {
        if rng.r#gen::<f64>() < mutation_rate {
            mutate_individual(individual, context, rng, enforce_order);
        }
    }
}

/// Mutates a single individual.
fn mutate_individual<R: Rng>(
    individual: &mut LayoutIndividual,
    context: &EvaluationContext,
    rng: &mut R,
    enforce_order: bool,
) {
    let mut tree = individual.tree().clone();
    mutate(&mut tree, rng, enforce_order);
    *individual = LayoutIndividual::from_tree(tree, context);
}

/// Mutates a slicing tree.
///
/// If `enforce_order` is true, performs cut-flip: picks a random internal node and toggles its cut (V ↔ H).
/// Otherwise, performs legacy leaf-swap: swaps photo indices of two random leaves.
///
/// The tree structure remains unchanged.
pub(crate) fn mutate<R: Rng>(tree: &mut SlicingTree, rng: &mut R, enforce_order: bool) {
    let n = tree.len();
    if n < 2 {
        return; // Nothing to mutate
    }

    if enforce_order {
        // Cut-flip: toggle one random internal node's cut type
        let internal_indices: Vec<u16> = tree
            .nodes()
            .iter()
            .enumerate()
            .filter_map(|(i, n)| if n.is_internal() { Some(i as u16) } else { None })
            .collect();

        if !internal_indices.is_empty() {
            let idx = internal_indices[rng.gen_range(0..internal_indices.len())];
            if let Node::Internal { cut, .. } = tree.node_mut(idx) {
                *cut = match cut {
                    Cut::V => Cut::H,
                    Cut::H => Cut::V,
                };
            }
        }
    } else {
        // Legacy behavior: leaf-swap
        let (leaf_indices, internal_indices) = collect_node_indices(tree);

        // Try to swap leaves first (more common), otherwise swap internals
        if leaf_indices.len() >= 2 {
            swap_random_leaves(tree, &leaf_indices, rng);
        } else if internal_indices.len() >= 2 {
            swap_random_internals(tree, &internal_indices, rng);
        }
    }
}

/// Collects indices of leaf nodes and internal nodes separately.
fn collect_node_indices(tree: &SlicingTree) -> (Vec<u16>, Vec<u16>) {
    let mut leaf_indices = Vec::new();
    let mut internal_indices = Vec::new();

    for (idx, node) in tree.nodes().iter().enumerate() {
        match node {
            Node::Leaf { .. } => leaf_indices.push(idx as u16),
            Node::Internal { .. } => internal_indices.push(idx as u16),
        }
    }

    (leaf_indices, internal_indices)
}

/// Swaps the photo indices of two randomly selected leaf nodes.
fn swap_random_leaves<R: Rng>(tree: &mut SlicingTree, leaf_indices: &[u16], rng: &mut R) {
    // Pick two different random leaves
    let i1 = rng.gen_range(0..leaf_indices.len());
    let mut i2 = rng.gen_range(0..leaf_indices.len());
    while i2 == i1 && leaf_indices.len() > 1 {
        i2 = rng.gen_range(0..leaf_indices.len());
    }

    let idx1 = leaf_indices[i1];
    let idx2 = leaf_indices[i2];

    // Extract photo indices
    let (p1, p2) = extract_photo_indices(tree, idx1, idx2);

    // Swap them
    set_photo_index(tree, idx1, p2);
    set_photo_index(tree, idx2, p1);
}

/// Extracts photo indices from two leaf nodes.
fn extract_photo_indices(tree: &SlicingTree, idx1: u16, idx2: u16) -> (u16, u16) {
    let p1 = match tree.node(idx1) {
        Node::Leaf { photo_idx, .. } => *photo_idx,
        _ => panic!("Expected leaf node"),
    };
    let p2 = match tree.node(idx2) {
        Node::Leaf { photo_idx, .. } => *photo_idx,
        _ => panic!("Expected leaf node"),
    };
    (p1, p2)
}

/// Sets the photo index of a leaf node.
fn set_photo_index(tree: &mut SlicingTree, idx: u16, photo_idx: u16) {
    if let Node::Leaf { photo_idx: p, .. } = tree.node_mut(idx) {
        *p = photo_idx;
    }
}

/// Swaps the cut types of two randomly selected internal nodes.
fn swap_random_internals<R: Rng>(tree: &mut SlicingTree, internal_indices: &[u16], rng: &mut R) {
    // Pick two different random internal nodes
    let i1 = rng.gen_range(0..internal_indices.len());
    let mut i2 = rng.gen_range(0..internal_indices.len());
    while i2 == i1 && internal_indices.len() > 1 {
        i2 = rng.gen_range(0..internal_indices.len());
    }

    let idx1 = internal_indices[i1];
    let idx2 = internal_indices[i2];

    // Extract cut types
    let (c1, c2) = extract_cut_types(tree, idx1, idx2);

    // Swap them
    set_cut_type(tree, idx1, c2);
    set_cut_type(tree, idx2, c1);
}

/// Extracts cut types from two internal nodes.
fn extract_cut_types(tree: &SlicingTree, idx1: u16, idx2: u16) -> (Cut, Cut) {
    let c1 = match tree.node(idx1) {
        Node::Internal { cut, .. } => *cut,
        _ => panic!("Expected internal node"),
    };
    let c2 = match tree.node(idx2) {
        Node::Internal { cut, .. } => *cut,
        _ => panic!("Expected internal node"),
    };
    (c1, c2)
}

/// Sets the cut type of an internal node.
fn set_cut_type(tree: &mut SlicingTree, idx: u16, cut: Cut) {
    if let Node::Internal { cut: c, .. } = tree.node_mut(idx) {
        *c = cut;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::page_layout_solver::tree::create::random_tree;
    use crate::solver::page_layout_solver::tree::validate::validate_tree;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn test_mutate_preserves_structure() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let mut tree = random_tree(5, &mut rng, true);

        let original_len = tree.len();
        let original_leaf_count = tree.leaf_count();

        mutate(&mut tree, &mut rng, true);

        // Structure unchanged
        assert_eq!(tree.len(), original_len);
        assert_eq!(tree.leaf_count(), original_leaf_count);

        // Tree still valid
        assert!(validate_tree(&tree).is_ok());
    }

    #[test]
    fn test_mutate_swaps_labels() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let mut tree = random_tree(4, &mut rng, true);

        // Collect photo indices before mutation
        let mut photo_indices_before: Vec<u16> = Vec::new();
        for node in tree.nodes() {
            if let Node::Leaf { photo_idx, .. } = node {
                photo_indices_before.push(*photo_idx);
            }
        }

        mutate(&mut tree, &mut rng, true);

        // Collect photo indices after mutation
        let mut photo_indices_after: Vec<u16> = Vec::new();
        for node in tree.nodes() {
            if let Node::Leaf { photo_idx, .. } = node {
                photo_indices_after.push(*photo_idx);
            }
        }

        // Same set of indices (might be in different order)
        photo_indices_before.sort_unstable();
        photo_indices_after.sort_unstable();
        assert_eq!(photo_indices_before, photo_indices_after);
    }

    #[test]
    fn test_mutate_multiple_times() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        for n in 2..=10 {
            let mut tree = random_tree(n, &mut rng, true);

            // Mutate 100 times
            for _ in 0..100 {
                mutate(&mut tree, &mut rng, true);
                assert!(validate_tree(&tree).is_ok());
            }
        }
    }

    #[test]
    fn test_mutate_single_photo_no_crash() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let mut tree = random_tree(1, &mut rng, true);

        // Should not crash on single-photo tree
        mutate(&mut tree, &mut rng, true);
        assert!(validate_tree(&tree).is_ok());
    }

    #[test]
    fn test_mutate_two_photos() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let mut tree = random_tree(2, &mut rng, true);

        mutate(&mut tree, &mut rng, true);

        // With 2 photos, mutation should swap them (if it targets leaves)
        assert!(validate_tree(&tree).is_ok());
    }
}
