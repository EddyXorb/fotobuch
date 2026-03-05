//! Slicing tree data structure for photo layout.
//!
//! A slicing tree is a full binary tree where:
//! - Leaf nodes represent photos
//! - Internal nodes represent cuts (V = vertical, H = horizontal)
//!
//! The tree is stored in an arena (Vec) for efficient cloning.

pub(super) mod build;
pub(super) mod crossover;
pub(super) mod mutate;
pub(super) mod validate;

use std::fmt;

/// Type of cut at an internal node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cut {
    /// Vertical cut: children are placed side-by-side (left/right).
    V,
    /// Horizontal cut: children are placed on top of each other (top/bottom).
    H,
}

/// Node in the slicing tree arena.
#[derive(Debug, Clone, Copy)]
pub enum Node {
    /// Leaf node representing a photo.
    Leaf {
        /// Index of the photo in the photo array.
        photo_idx: u16,
        /// Index of parent node (None for root).
        parent: Option<u16>,
    },
    /// Internal node representing a cut.
    Internal {
        /// Type of cut (V or H).
        cut: Cut,
        /// Index of left child.
        left: u16,
        /// Index of right child.
        right: u16,
        /// Index of parent node (None for root).
        parent: Option<u16>,
    },
}

impl Node {
    /// Returns the parent index, if any.
    pub fn parent(&self) -> Option<u16> {
        match self {
            Node::Leaf { parent, .. } => *parent,
            Node::Internal { parent, .. } => *parent,
        }
    }
    
    /// Returns whether this node is a leaf.
    pub fn is_leaf(&self) -> bool {
        matches!(self, Node::Leaf { .. })
    }
    
    /// Returns whether this node is internal.
    pub fn is_internal(&self) -> bool {
        matches!(self, Node::Internal { .. })
    }
}

/// Slicing tree stored in an arena (Vec) for efficient cloning.
///
/// For N photos, the tree has exactly N leaves and N-1 internal nodes,
/// for a total of 2N-1 nodes. The root is always at index 0.
#[derive(Clone)]
pub struct SlicingTree {
    /// Arena storage for all nodes. Root is always nodes[0].
    nodes: Vec<Node>,
}

impl SlicingTree {
    /// Creates a new slicing tree from a vector of nodes.
    ///
    /// # Panics
    ///
    /// Panics if the nodes vector is empty.
    pub fn new(nodes: Vec<Node>) -> Self {
        assert!(!nodes.is_empty(), "SlicingTree cannot be empty");
        Self { nodes }
    }
    
    /// Returns the number of nodes in the tree.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    
    /// Returns whether the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    
    /// Returns a reference to the node at the given index.
    pub fn node(&self, idx: u16) -> &Node {
        &self.nodes[idx as usize]
    }
    
    /// Returns a mutable reference to the node at the given index.
    pub fn node_mut(&mut self, idx: u16) -> &mut Node {
        &mut self.nodes[idx as usize]
    }
    
    /// Returns a slice of all nodes.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    
    /// Returns the root node (always at index 0).
    #[allow(dead_code)]
    pub fn root(&self) -> &Node {
        &self.nodes[0]
    }
    
    /// Returns the number of leaf nodes in the tree.
    pub fn leaf_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_leaf()).count()
    }
    
    /// Returns the number of internal nodes in the tree.
    pub fn internal_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_internal()).count()
    }
    
    /// Visits all nodes in the tree in depth-first order, calling the visitor function.
    #[allow(dead_code)]
    pub fn visit<F>(&self, mut visitor: F)
    where
        F: FnMut(u16, &Node),
    {
        self.visit_recursive(0, &mut visitor);
    }
    
    #[allow(dead_code)]
    fn visit_recursive<F>(&self, idx: u16, visitor: &mut F)
    where
        F: FnMut(u16, &Node),
    {
        let node = &self.nodes[idx as usize];
        visitor(idx, node);
        
        if let Node::Internal { left, right, .. } = node {
            self.visit_recursive(*left, visitor);
            self.visit_recursive(*right, visitor);
        }
    }
}

impl fmt::Debug for SlicingTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SlicingTree")
            .field("nodes", &self.nodes.len())
            .field("leaves", &self.leaf_count())
            .field("internal", &self.internal_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cut_enum() {
        assert_ne!(Cut::V, Cut::H);
    }
    
    #[test]
    fn test_node_parent() {
        let leaf = Node::Leaf {
            photo_idx: 0,
            parent: Some(1),
        };
        assert_eq!(leaf.parent(), Some(1));
        assert!(leaf.is_leaf());
        assert!(!leaf.is_internal());
        
        let internal = Node::Internal {
            cut: Cut::V,
            left: 1,
            right: 2,
            parent: None,
        };
        assert_eq!(internal.parent(), None);
        assert!(!internal.is_leaf());
        assert!(internal.is_internal());
    }
    
    #[test]
    fn test_slicing_tree_simple() {
        // Tree with 2 photos: root is Internal(V), children are leaves
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
        assert_eq!(tree.len(), 3);
        assert_eq!(tree.leaf_count(), 2);
        assert_eq!(tree.internal_count(), 1);
        
        let root = tree.root();
        assert!(root.is_internal());
        assert_eq!(root.parent(), None);
    }
    
    #[test]
    #[should_panic(expected = "SlicingTree cannot be empty")]
    fn test_slicing_tree_empty() {
        SlicingTree::new(vec![]);
    }
    
    #[test]
    fn test_tree_visit() {
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
        let mut visited = Vec::new();
        tree.visit(|idx, _| visited.push(idx));
        
        assert_eq!(visited, vec![0, 1, 2]);
    }
}
