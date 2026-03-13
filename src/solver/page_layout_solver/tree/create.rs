//! Functions for building random slicing trees.

use super::{Cut, Node, SlicingTree};
use rand::Rng;

/// Assigns photo indices to tree leaves in DFS preorder.
///
/// Ensures photos are visited in reading order (left-to-right, top-to-bottom
/// according to tree structure). Photo index 0 gets the oldest photo, 1 the next, etc.
fn assign_photos_by_dfs(tree: &mut SlicingTree) {
    let mut counter = 0u16;
    assign_recursive(tree, 0, &mut counter);
}

/// Recursively assigns photo indices during DFS traversal.
fn assign_recursive(tree: &mut SlicingTree, idx: u16, counter: &mut u16) {
    match *tree.node(idx) {
        Node::Leaf { .. } => {
            if let Node::Leaf { photo_idx, .. } = tree.node_mut(idx) {
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

/// Generates a random slicing tree for N photos.
///
/// Algorithm:
/// 1. Start with a single leaf (photo 0)
/// 2. N-1 times: replace a random leaf with an internal node with two new leaves
/// 3. If `enforce_order` is true, assign photos in DFS preorder.
///    Otherwise, shuffle photo indices randomly across all leaves.
///
/// Returns a tree with N leaves and N-1 internal nodes.
pub(crate) fn random_tree<R: Rng>(n: usize, rng: &mut R, enforce_order: bool) -> SlicingTree {
    assert!(n > 0, "Cannot create tree with 0 photos");

    if n == 1 {
        // Single photo: just one leaf node
        let nodes = vec![Node::Leaf {
            photo_idx: 0,
            parent: None,
        }];
        return SlicingTree::new(nodes);
    }

    // Start with a single leaf
    let mut nodes = vec![Node::Leaf {
        photo_idx: 0,
        parent: None,
    }];

    // Track which nodes are leaves (by index)
    let mut leaves: Vec<u16> = vec![0];

    // Add N-1 internal nodes
    for _ in 0..n - 1 {
        // Pick a random leaf to replace
        let leaf_pos = rng.gen_range(0..leaves.len());
        let leaf_idx = leaves[leaf_pos];

        // Remember the old parent before we overwrite this node
        let old_parent = nodes[leaf_idx as usize].parent();

        // Create two new leaf children
        let left_idx = nodes.len() as u16;
        let right_idx = left_idx + 1;

        nodes.push(Node::Leaf {
            photo_idx: 0, // Will be assigned later
            parent: Some(leaf_idx),
        });
        nodes.push(Node::Leaf {
            photo_idx: 0, // Will be assigned later
            parent: Some(leaf_idx),
        });

        // Replace the old leaf with an internal node
        let cut = if rng.gen_bool(0.5) { Cut::V } else { Cut::H };
        nodes[leaf_idx as usize] = Node::Internal {
            cut,
            left: left_idx,
            right: right_idx,
            parent: old_parent,
        };

        // Update leaves list: remove the replaced leaf, add the two new ones
        leaves.swap_remove(leaf_pos);
        leaves.push(left_idx);
        leaves.push(right_idx);
    }

    // Now assign photo indices to all leaves
    if enforce_order {
        // Create tree and assign photos in DFS preorder for deterministic reading order
        let mut tree = SlicingTree::new(nodes);
        assign_photos_by_dfs(&mut tree);
        tree
    } else {
        // Fisher-Yates shuffle for random assignment (legacy behavior)
        let mut photo_indices: Vec<u16> = (0..n as u16).collect();
        for i in (1..photo_indices.len()).rev() {
            let j = rng.gen_range(0..=i);
            photo_indices.swap(i, j);
        }

        let mut photo_iter = photo_indices.into_iter();
        for node in &mut nodes {
            if let Node::Leaf { photo_idx, .. } = node {
                *photo_idx = photo_iter
                    .next()
                    .expect("Photo count should match leaf count");
            }
        }

        SlicingTree::new(nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn test_random_tree_single_photo() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let tree = random_tree(1, &mut rng, true);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree.leaf_count(), 1);
        assert_eq!(tree.internal_count(), 0);

        match tree.root() {
            Node::Leaf { photo_idx, parent } => {
                assert_eq!(*photo_idx, 0);
                assert_eq!(*parent, None);
            }
            _ => panic!("Root should be a leaf"),
        }
    }

    #[test]
    fn test_random_tree_two_photos() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let tree = random_tree(2, &mut rng, true);

        assert_eq!(tree.len(), 3);
        assert_eq!(tree.leaf_count(), 2);
        assert_eq!(tree.internal_count(), 1);

        assert!(tree.root().is_internal());
    }

    #[test]
    fn test_random_tree_sizes() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        for n in 2..=20 {
            let tree = random_tree(n, &mut rng, true);
            assert_eq!(tree.len(), 2 * n - 1);
            assert_eq!(tree.leaf_count(), n);
            assert_eq!(tree.internal_count(), n - 1);
        }
    }

    #[test]
    fn test_random_tree_photo_indices() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let n = 5;
        let tree = random_tree(n, &mut rng, false);

        // Collect all photo indices from leaves
        let mut photo_indices: Vec<u16> = Vec::new();
        tree.visit(|_, node| {
            if let Node::Leaf { photo_idx, .. } = node {
                photo_indices.push(*photo_idx);
            }
        });

        // Should be a permutation of 0..n
        photo_indices.sort_unstable();
        assert_eq!(photo_indices, (0..n as u16).collect::<Vec<_>>());
    }

    #[test]
    fn test_random_tree_deterministic() {
        let mut rng1 = ChaCha8Rng::seed_from_u64(12345);
        let mut rng2 = ChaCha8Rng::seed_from_u64(12345);

        let tree1 = random_tree(10, &mut rng1, true);
        let tree2 = random_tree(10, &mut rng2, true);

        // Same seed should produce identical trees
        assert_eq!(tree1.len(), tree2.len());

        for i in 0..tree1.len() {
            let n1 = tree1.node(i as u16);
            let n2 = tree2.node(i as u16);

            match (n1, n2) {
                (
                    Node::Leaf {
                        photo_idx: p1,
                        parent: par1,
                    },
                    Node::Leaf {
                        photo_idx: p2,
                        parent: par2,
                    },
                ) => {
                    assert_eq!(p1, p2);
                    assert_eq!(par1, par2);
                }
                (
                    Node::Internal {
                        cut: c1,
                        left: l1,
                        right: r1,
                        parent: par1,
                    },
                    Node::Internal {
                        cut: c2,
                        left: l2,
                        right: r2,
                        parent: par2,
                    },
                ) => {
                    assert_eq!(c1, c2);
                    assert_eq!(l1, l2);
                    assert_eq!(r1, r2);
                    assert_eq!(par1, par2);
                }
                _ => panic!("Node types differ at index {}", i),
            }
        }
    }

    #[test]
    #[should_panic(expected = "Cannot create tree with 0 photos")]
    fn test_random_tree_zero_photos() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        random_tree(0, &mut rng, true);
    }
}
