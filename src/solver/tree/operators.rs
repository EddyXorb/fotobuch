//! Genetic operators for slicing trees: mutation and crossover.

use super::{Node, SlicingTree};
use rand::Rng;

/// Mutates a slicing tree by swapping the labels of two random nodes of the same type.
///
/// If both nodes are leaves, swap their photo_idx.
/// If both nodes are internal, swap their cut type.
///
/// The tree structure remains unchanged.
pub fn mutate<R: Rng>(tree: &mut SlicingTree, rng: &mut R) {
    let n = tree.len();
    if n < 2 {
        return; // Nothing to swap
    }

    // Collect indices of leaves and internal nodes
    let mut leaf_indices = Vec::new();
    let mut internal_indices = Vec::new();

    for (idx, node) in tree.nodes().iter().enumerate() {
        match node {
            Node::Leaf { .. } => leaf_indices.push(idx as u16),
            Node::Internal { .. } => internal_indices.push(idx as u16),
        }
    }

    // Try to swap leaves first (more common), otherwise swap internals
    if leaf_indices.len() >= 2 {
        // Pick two different random leaves
        let i1 = rng.gen_range(0..leaf_indices.len());
        let mut i2 = rng.gen_range(0..leaf_indices.len());
        while i2 == i1 && leaf_indices.len() > 1 {
            i2 = rng.gen_range(0..leaf_indices.len());
        }

        let idx1 = leaf_indices[i1];
        let idx2 = leaf_indices[i2];

        // Extract photo indices
        let p1 = match tree.node(idx1) {
            Node::Leaf { photo_idx, .. } => *photo_idx,
            _ => return,
        };
        let p2 = match tree.node(idx2) {
            Node::Leaf { photo_idx, .. } => *photo_idx,
            _ => return,
        };

        // Swap them
        if let Node::Leaf { photo_idx, .. } = tree.node_mut(idx1) {
            *photo_idx = p2;
        }
        if let Node::Leaf { photo_idx, .. } = tree.node_mut(idx2) {
            *photo_idx = p1;
        }
    } else if internal_indices.len() >= 2 {
        // Pick two different random internal nodes
        let i1 = rng.gen_range(0..internal_indices.len());
        let mut i2 = rng.gen_range(0..internal_indices.len());
        while i2 == i1 && internal_indices.len() > 1 {
            i2 = rng.gen_range(0..internal_indices.len());
        }

        let idx1 = internal_indices[i1];
        let idx2 = internal_indices[i2];

        // Extract cut types
        let c1 = match tree.node(idx1) {
            Node::Internal { cut, .. } => *cut,
            _ => return,
        };
        let c2 = match tree.node(idx2) {
            Node::Internal { cut, .. } => *cut,
            _ => return,
        };

        // Swap them
        if let Node::Internal { cut, .. } = tree.node_mut(idx1) {
            *cut = c2;
        }
        if let Node::Internal { cut, .. } = tree.node_mut(idx2) {
            *cut = c1;
        }
    }
}

/// Performs crossover between two slicing trees.
///
/// Finds subtrees with equal leaf counts >= 3, swaps them.
/// Returns two new trees, or None if no compatible subtrees found.
pub fn crossover<R: Rng>(
    _a: &SlicingTree,
    _b: &SlicingTree,
    _rng: &mut R,
) -> Option<(SlicingTree, SlicingTree)> {
    // TODO: Implement in Step 5
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::tree::{random_tree, validate_tree};
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn test_mutate_preserves_structure() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let mut tree = random_tree(5, &mut rng);

        let original_len = tree.len();
        let original_leaf_count = tree.leaf_count();

        mutate(&mut tree, &mut rng);

        // Structure unchanged
        assert_eq!(tree.len(), original_len);
        assert_eq!(tree.leaf_count(), original_leaf_count);

        // Tree still valid
        assert!(validate_tree(&tree).is_ok());
    }

    #[test]
    fn test_mutate_swaps_labels() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let mut tree = random_tree(4, &mut rng);

        // Collect photo indices before mutation
        let mut photo_indices_before: Vec<u16> = Vec::new();
        for node in tree.nodes() {
            if let Node::Leaf { photo_idx, .. } = node {
                photo_indices_before.push(*photo_idx);
            }
        }

        mutate(&mut tree, &mut rng);

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
            let mut tree = random_tree(n, &mut rng);

            // Mutate 100 times
            for _ in 0..100 {
                mutate(&mut tree, &mut rng);
                assert!(validate_tree(&tree).is_ok());
            }
        }
    }

    #[test]
    fn test_mutate_single_photo_no_crash() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let mut tree = random_tree(1, &mut rng);

        // Should not crash on single-photo tree
        mutate(&mut tree, &mut rng);
        assert!(validate_tree(&tree).is_ok());
    }

    #[test]
    fn test_mutate_two_photos() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let mut tree = random_tree(2, &mut rng);

        mutate(&mut tree, &mut rng);

        // With 2 photos, mutation should swap them (if it targets leaves)
        assert!(validate_tree(&tree).is_ok());
    }
}
