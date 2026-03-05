//! Property-based tests for tree operations.

#[cfg(test)]
mod tests {
    use crate::solver::page_layout_solver::tree::{
        build::random_tree, crossover::crossover, mutate::mutate, validate::validate_tree,
        SlicingTree,
    };
    use proptest::prelude::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use std::collections::HashSet;

    // Strategy for generating tree sizes
    fn tree_size() -> impl Strategy<Value = usize> {
        1usize..=20
    }

    // Strategy for generating a valid slicing tree
    fn arb_tree() -> impl Strategy<Value = (SlicingTree, usize)> {
        tree_size().prop_flat_map(|n| {
            // Generate a seed for reproducibility within proptest
            any::<u64>().prop_map(move |seed| {
                let mut rng = ChaCha8Rng::seed_from_u64(seed);
                let tree = random_tree(n, &mut rng);
                (tree, n)
            })
        })
    }

    // Strategy for generating two trees (for crossover tests)
    fn arb_two_trees() -> impl Strategy<Value = (SlicingTree, SlicingTree, usize)> {
        tree_size().prop_flat_map(|n| {
            any::<(u64, u64)>().prop_map(move |(seed1, seed2)| {
                let mut rng1 = ChaCha8Rng::seed_from_u64(seed1);
                let mut rng2 = ChaCha8Rng::seed_from_u64(seed2);
                let tree1 = random_tree(n, &mut rng1);
                let tree2 = random_tree(n, &mut rng2);
                (tree1, tree2, n)
            })
        })
    }

    // Helper function to extract photo indices from a tree
    fn get_photo_indices(tree: &SlicingTree) -> Vec<u16> {
        let mut indices: Vec<u16> = tree
            .nodes()
            .iter()
            .filter_map(|node| {
                if let crate::solver::page_layout_solver::tree::Node::Leaf { photo_idx, .. } = node
                {
                    Some(*photo_idx)
                } else {
                    None
                }
            })
            .collect();
        indices.sort_unstable();
        indices
    }

    proptest! {
        /// Property: random_tree always produces a valid tree
        #[test]
        fn prop_random_tree_is_valid(n in tree_size(), seed in any::<u64>()) {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let tree = random_tree(n, &mut rng);

            // The tree should always be valid
            prop_assert!(validate_tree(&tree).is_ok(), "Tree is invalid: {:?}", validate_tree(&tree));
        }

        /// Property: random_tree produces exactly N leaves
        #[test]
        fn prop_random_tree_has_n_leaves(n in tree_size(), seed in any::<u64>()) {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let tree = random_tree(n, &mut rng);

            prop_assert_eq!(tree.leaf_count(), n, "Expected {} leaves, got {}", n, tree.leaf_count());
        }

        /// Property: random_tree produces exactly N-1 internal nodes (for N > 1)
        #[test]
        fn prop_random_tree_has_n_minus_1_internal(n in tree_size(), seed in any::<u64>()) {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let tree = random_tree(n, &mut rng);

            let expected_internal = if n == 1 { 0 } else { n - 1 };
            prop_assert_eq!(tree.internal_count(), expected_internal,
                "Expected {} internal nodes, got {}", expected_internal, tree.internal_count());
        }

        /// Property: random_tree produces all unique photo indices from 0 to N-1
        #[test]
        fn prop_random_tree_has_all_photos(n in tree_size(), seed in any::<u64>()) {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let tree = random_tree(n, &mut rng);

            let photo_indices = get_photo_indices(&tree);
            let expected: Vec<u16> = (0..n as u16).collect();

            prop_assert_eq!(photo_indices, expected,
                "Photo indices should be a permutation of 0..{}", n);
        }

        /// Property: mutate preserves tree validity
        #[test]
        fn prop_mutate_preserves_validity((tree, _n) in arb_tree(), seed in any::<u64>()) {
            let mut tree = tree;
            let mut rng = ChaCha8Rng::seed_from_u64(seed);

            mutate(&mut tree, &mut rng);

            prop_assert!(validate_tree(&tree).is_ok(), "Tree is invalid after mutation: {:?}", validate_tree(&tree));
        }

        /// Property: mutate preserves the number of leaves
        #[test]
        fn prop_mutate_preserves_leaf_count((tree, n) in arb_tree(), seed in any::<u64>()) {
            let original_leaves = tree.leaf_count();
            let mut tree = tree;
            let mut rng = ChaCha8Rng::seed_from_u64(seed);

            mutate(&mut tree, &mut rng);

            prop_assert_eq!(tree.leaf_count(), original_leaves,
                "Leaf count changed from {} to {} (expected n={})", original_leaves, tree.leaf_count(), n);
        }

        /// Property: mutate preserves the number of internal nodes
        #[test]
        fn prop_mutate_preserves_internal_count((tree, _n) in arb_tree(), seed in any::<u64>()) {
            let original_internal = tree.internal_count();
            let mut tree = tree;
            let mut rng = ChaCha8Rng::seed_from_u64(seed);

            mutate(&mut tree, &mut rng);

            prop_assert_eq!(tree.internal_count(), original_internal,
                "Internal count changed from {} to {}", original_internal, tree.internal_count());
        }

        /// Property: mutate preserves the set of photo indices
        #[test]
        fn prop_mutate_preserves_photo_indices((tree, _n) in arb_tree(), seed in any::<u64>()) {
            let original_photos = get_photo_indices(&tree);
            let mut tree = tree;
            let mut rng = ChaCha8Rng::seed_from_u64(seed);

            mutate(&mut tree, &mut rng);

            let new_photos = get_photo_indices(&tree);
            prop_assert_eq!(original_photos, new_photos,
                "Photo indices changed after mutation");
        }

        /// Property: mutate preserves tree structure (node count)
        #[test]
        fn prop_mutate_preserves_node_count((tree, _n) in arb_tree(), seed in any::<u64>()) {
            let original_len = tree.len();
            let mut tree = tree;
            let mut rng = ChaCha8Rng::seed_from_u64(seed);

            mutate(&mut tree, &mut rng);

            prop_assert_eq!(tree.len(), original_len,
                "Node count changed from {} to {}", original_len, tree.len());
        }

        /// Property: crossover (when successful) produces two valid trees
        #[test]
        fn prop_crossover_preserves_validity((tree_a, tree_b, _n) in arb_two_trees(), seed in any::<u64>()) {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);

            if let Some((new_a, new_b)) = crossover(&tree_a, &tree_b, &mut rng) {
                prop_assert!(validate_tree(&new_a).is_ok(),
                    "Tree A is invalid after crossover: {:?}", validate_tree(&new_a));
                prop_assert!(validate_tree(&new_b).is_ok(),
                    "Tree B is invalid after crossover: {:?}", validate_tree(&new_b));
            }
            // If crossover returns None, that's fine (no compatible subtrees)
        }

        /// Property: crossover preserves the number of leaves in both trees
        #[test]
        fn prop_crossover_preserves_leaf_counts((tree_a, tree_b, _n) in arb_two_trees(), seed in any::<u64>()) {
            let leaves_a = tree_a.leaf_count();
            let leaves_b = tree_b.leaf_count();
            let mut rng = ChaCha8Rng::seed_from_u64(seed);

            if let Some((new_a, new_b)) = crossover(&tree_a, &tree_b, &mut rng) {
                prop_assert_eq!(new_a.leaf_count(), leaves_a,
                    "Tree A leaf count changed from {} to {}", leaves_a, new_a.leaf_count());
                prop_assert_eq!(new_b.leaf_count(), leaves_b,
                    "Tree B leaf count changed from {} to {}", leaves_b, new_b.leaf_count());
            }
        }

        /// Property: crossover preserves photo indices in each tree
        #[test]
        fn prop_crossover_preserves_photo_indices((tree_a, tree_b, _n) in arb_two_trees(), seed in any::<u64>()) {
            let photos_a = get_photo_indices(&tree_a);
            let photos_b = get_photo_indices(&tree_b);
            let mut rng = ChaCha8Rng::seed_from_u64(seed);

            if let Some((new_a, new_b)) = crossover(&tree_a, &tree_b, &mut rng) {
                let new_photos_a = get_photo_indices(&new_a);
                let new_photos_b = get_photo_indices(&new_b);

                prop_assert_eq!(photos_a, new_photos_a,
                    "Tree A photo indices changed after crossover");
                prop_assert_eq!(photos_b, new_photos_b,
                    "Tree B photo indices changed after crossover");
            }
        }

        /// Property: multiple mutations preserve validity
        #[test]
        fn prop_multiple_mutations_preserve_validity((tree, _n) in arb_tree(), seed in any::<u64>(), count in 1usize..=10) {
            let mut tree = tree;
            let mut rng = ChaCha8Rng::seed_from_u64(seed);

            for _ in 0..count {
                mutate(&mut tree, &mut rng);
            }

            prop_assert!(validate_tree(&tree).is_ok(),
                "Tree is invalid after {} mutations: {:?}", count, validate_tree(&tree));
        }

        /// Property: crossover followed by mutation preserves validity
        #[test]
        fn prop_crossover_then_mutate_preserves_validity(
            (tree_a, tree_b, _n) in arb_two_trees(),
            seed1 in any::<u64>(),
            seed2 in any::<u64>()
        ) {
            let mut rng1 = ChaCha8Rng::seed_from_u64(seed1);

            if let Some((mut new_a, mut new_b)) = crossover(&tree_a, &tree_b, &mut rng1) {
                let mut rng2 = ChaCha8Rng::seed_from_u64(seed2);
                mutate(&mut new_a, &mut rng2);
                mutate(&mut new_b, &mut rng2);

                prop_assert!(validate_tree(&new_a).is_ok(),
                    "Tree A is invalid after crossover + mutation: {:?}", validate_tree(&new_a));
                prop_assert!(validate_tree(&new_b).is_ok(),
                    "Tree B is invalid after crossover + mutation: {:?}", validate_tree(&new_b));
            }
        }

        /// Property: all leaf nodes have unique photo indices within a tree
        #[test]
        fn prop_unique_photo_indices((tree, _n) in arb_tree()) {
            let photo_indices = get_photo_indices(&tree);
            let unique_indices: HashSet<u16> = photo_indices.iter().copied().collect();

            prop_assert_eq!(photo_indices.len(), unique_indices.len(),
                "Duplicate photo indices found: {:?}", photo_indices);
        }

        /// Property: tree has exactly 2N-1 nodes (or 1 for N=1)
        #[test]
        fn prop_tree_has_correct_node_count((tree, n) in arb_tree()) {
            let expected = if n == 1 { 1 } else { 2 * n - 1 };
            prop_assert_eq!(tree.len(), expected,
                "Expected {} nodes for {} photos, got {}", expected, n, tree.len());
        }

        /// Property: root node always has parent=None
        #[test]
        fn prop_root_has_no_parent((tree, _n) in arb_tree()) {
            prop_assert!(tree.root().parent().is_none(),
                "Root node has a parent: {:?}", tree.root().parent());
        }
    }
}
