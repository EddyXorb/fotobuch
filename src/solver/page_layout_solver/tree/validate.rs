//! Validation functions for slicing trees.

#[cfg(test)]
use super::{Node, SlicingTree};

/// Validates that a slicing tree satisfies all invariants.
///
/// Checks:
/// 1. Exactly N leaves and N-1 internal nodes (2N-1 total)
/// 2. Each photo_idx appears exactly once (permutation of 0..N)
/// 3. All left/right indices point to valid nodes
/// 4. All parent references are consistent
/// 5. Root has parent=None
///
/// Returns Ok(()) if valid, Err(String) with error message otherwise.
#[cfg(test)]
pub(crate) fn validate_tree(tree: &SlicingTree) -> Result<(), String> {
    if tree.is_empty() {
        return Err("Tree is empty".to_string());
    }

    let n_leaves = tree.leaf_count();
    let n_internal = tree.internal_count();

    // Check node counts
    if n_leaves == 0 {
        return Err("Tree has no leaves".to_string());
    }

    if n_leaves == 1 {
        // Single leaf: should be the only node
        if tree.len() != 1 {
            return Err(format!(
                "Single-leaf tree should have 1 node, but has {}",
                tree.len()
            ));
        }
    } else {
        // Multiple leaves: should have N leaves and N-1 internal
        if n_internal != n_leaves - 1 {
            return Err(format!(
                "Expected {} internal nodes for {} leaves, but found {}",
                n_leaves - 1,
                n_leaves,
                n_internal
            ));
        }
    }

    // Root must have parent=None
    if tree.root().parent().is_some() {
        return Err("Root node has a parent".to_string());
    }

    // Collect photo indices and check for duplicates
    let mut photo_indices = Vec::new();
    for (idx, node) in tree.nodes().iter().enumerate() {
        match node {
            Node::Leaf { photo_idx, parent } => {
                photo_indices.push(*photo_idx);

                // Check parent reference
                if let Some(parent_idx) = parent {
                    if *parent_idx as usize >= tree.len() {
                        return Err(format!(
                            "Leaf at index {} has invalid parent {}",
                            idx, parent_idx
                        ));
                    }

                    // Parent must be an internal node
                    if !tree.node(*parent_idx).is_internal() {
                        return Err(format!(
                            "Leaf at index {} has parent {} which is not internal",
                            idx, parent_idx
                        ));
                    }
                }
            }
            Node::Internal {
                left,
                right,
                parent,
                ..
            } => {
                // Check child indices
                if *left as usize >= tree.len() {
                    return Err(format!(
                        "Internal node at index {} has invalid left child {}",
                        idx, left
                    ));
                }
                if *right as usize >= tree.len() {
                    return Err(format!(
                        "Internal node at index {} has invalid right child {}",
                        idx, right
                    ));
                }

                // Check parent reference
                if let Some(parent_idx) = parent {
                    if *parent_idx as usize >= tree.len() {
                        return Err(format!(
                            "Internal node at index {} has invalid parent {}",
                            idx, parent_idx
                        ));
                    }

                    // Parent must be an internal node
                    if !tree.node(*parent_idx).is_internal() {
                        return Err(format!(
                            "Internal node at index {} has parent {} which is not internal",
                            idx, parent_idx
                        ));
                    }
                }
            }
        }
    }

    // Check that photo indices form a permutation of 0..N
    photo_indices.sort_unstable();
    let expected: Vec<u16> = (0..n_leaves as u16).collect();
    if photo_indices != expected {
        return Err(format!(
            "Photo indices are not a permutation of 0..{}: got {:?}",
            n_leaves, photo_indices
        ));
    }

    // Verify parent-child consistency
    for (idx, node) in tree.nodes().iter().enumerate() {
        if let Node::Internal { left, right, .. } = node {
            let left_parent = tree.node(*left).parent();
            let right_parent = tree.node(*right).parent();

            if left_parent != Some(idx as u16) {
                return Err(format!(
                    "Internal node at {} has left child {} with parent {:?}, expected Some({})",
                    idx, left, left_parent, idx
                ));
            }

            if right_parent != Some(idx as u16) {
                return Err(format!(
                    "Internal node at {} has right child {} with parent {:?}, expected Some({})",
                    idx, right, right_parent, idx
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::page_layout_solver::tree::create::random_tree;
    use crate::solver::page_layout_solver::tree::Cut;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn test_validate_simple_tree() {
        let nodes = vec![
            Node::Internal {
                cut: Cut::V,
                left: 1,
                right: 2,
                parent: None,
            },
            Node::Leaf {
                photo_idx: 0,
                parent: Some(0),
            },
            Node::Leaf {
                photo_idx: 1,
                parent: Some(0),
            },
        ];
        let tree = SlicingTree::new(nodes);
        assert!(validate_tree(&tree).is_ok());
    }

    #[test]
    fn test_validate_single_leaf() {
        let nodes = vec![Node::Leaf {
            photo_idx: 0,
            parent: None,
        }];
        let tree = SlicingTree::new(nodes);
        assert!(validate_tree(&tree).is_ok());
    }

    #[test]
    fn test_validate_invalid_root_parent() {
        let nodes = vec![
            Node::Internal {
                cut: Cut::V,
                left: 1,
                right: 2,
                parent: Some(999), // Invalid!
            },
            Node::Leaf {
                photo_idx: 0,
                parent: Some(0),
            },
            Node::Leaf {
                photo_idx: 1,
                parent: Some(0),
            },
        ];
        let tree = SlicingTree::new(nodes);
        assert!(validate_tree(&tree).is_err());
    }

    #[test]
    fn test_validate_duplicate_photo_indices() {
        let nodes = vec![
            Node::Internal {
                cut: Cut::V,
                left: 1,
                right: 2,
                parent: None,
            },
            Node::Leaf {
                photo_idx: 0,
                parent: Some(0),
            },
            Node::Leaf {
                photo_idx: 0, // Duplicate!
                parent: Some(0),
            },
        ];
        let tree = SlicingTree::new(nodes);
        assert!(validate_tree(&tree).is_err());
    }

    #[test]
    fn test_validate_invalid_child_index() {
        let nodes = vec![
            Node::Internal {
                cut: Cut::V,
                left: 1,
                right: 999, // Invalid!
                parent: None,
            },
            Node::Leaf {
                photo_idx: 0,
                parent: Some(0),
            },
        ];
        let tree = SlicingTree::new(nodes);
        assert!(validate_tree(&tree).is_err());
    }

    #[test]
    fn test_validate_inconsistent_parent() {
        let nodes = vec![
            Node::Internal {
                cut: Cut::V,
                left: 1,
                right: 2,
                parent: None,
            },
            Node::Leaf {
                photo_idx: 0,
                parent: Some(999), // Wrong parent!
            },
            Node::Leaf {
                photo_idx: 1,
                parent: Some(0),
            },
        ];
        let tree = SlicingTree::new(nodes);
        assert!(validate_tree(&tree).is_err());
    }

    #[test]
    fn test_validate_random_trees() {
        for seed in 0..100 {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            for n in 1..=20 {
                let tree = random_tree(n, &mut rng);
                assert!(
                    validate_tree(&tree).is_ok(),
                    "Random tree with {} photos failed validation",
                    n
                );
            }
        }
    }

    #[test]
    fn test_validate_wrong_node_count() {
        // 3 nodes but only 1 leaf (should have 2 leaves for 1 internal)
        let nodes = vec![
            Node::Internal {
                cut: Cut::V,
                left: 1,
                right: 2,
                parent: None,
            },
            Node::Leaf {
                photo_idx: 0,
                parent: Some(0),
            },
            Node::Internal {
                cut: Cut::H,
                left: 1,
                right: 1,
                parent: Some(0),
            },
        ];
        let tree = SlicingTree::new(nodes);
        assert!(validate_tree(&tree).is_err());
    }
}
