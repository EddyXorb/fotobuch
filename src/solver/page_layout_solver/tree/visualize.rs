//! Visualization of slicing trees for debugging.
//!
//! Generates SVG files showing the tree structure with cuts and photo indices.

use super::super::tree::{Cut, Node, SlicingTree};
use std::fmt::{self, Write};
use std::fs;
use std::path::Path;

// Layout constants for tree visualization
const NODE_RADIUS: f64 = 25.0;
const LEVEL_HEIGHT: f64 = 80.0;
const MIN_H_SPACING: f64 = 60.0;
const CANVAS_PADDING: f64 = 20.0;

/// Layout information for a node in the visualization.
#[derive(Debug, Clone, Copy)]
struct NodeLayout {
    x: f64,
    y: f64,
    subtree_width: f64,
}

/// Generates an SVG visualization of the slicing tree.
///
/// The tree is drawn top-down with the root at the top.
/// Internal nodes show the cut type (V/H), leaf nodes show photo indices.
///
/// # Arguments
///
/// * `tree` - The slicing tree to visualize
/// * `output_path` - Path where the SVG file will be written
///
/// # Returns
///
/// Returns `Ok(())` if successful, or an error if file writing fails.
pub fn visualize_tree<P: AsRef<Path>>(
    tree: &SlicingTree,
    output_path: P,
) -> std::io::Result<()> {
    let svg = generate_svg(tree)
        .map_err(std::io::Error::other)?;
    fs::write(output_path, svg)
}

/// Generates SVG content for the tree.
fn generate_svg(tree: &SlicingTree) -> Result<String, fmt::Error> {
    // Calculate layout for all nodes
    let layouts = calculate_layouts(tree, NODE_RADIUS, MIN_H_SPACING);
    
    // Calculate canvas dimensions
    let max_x = layouts
        .iter()
        .map(|l| l.x + l.subtree_width / 2.0)
        .fold(0.0, f64::max);
    let max_depth = calculate_depth(tree, 0);
    let canvas_width = max_x + NODE_RADIUS * 2.0 + CANVAS_PADDING;
    let canvas_height = (max_depth as f64) * LEVEL_HEIGHT + NODE_RADIUS * 2.0 + CANVAS_PADDING;
    
    let mut svg = String::new();
    
    // SVG header
    writeln!(
        &mut svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"#,
        canvas_width, canvas_height, canvas_width, canvas_height
    )?;
    
    // Style definitions
    writeln!(&mut svg, r#"<style>
        .internal-node {{ fill: #4A90E2; stroke: #2E5C8A; stroke-width: 2; }}
        .leaf-node {{ fill: #7ED321; stroke: #5FA319; stroke-width: 2; }}
        .node-text {{ fill: white; font-family: Arial, sans-serif; font-size: 14px; font-weight: bold; text-anchor: middle; dominant-baseline: middle; }}
        .edge {{ stroke: #333; stroke-width: 2; fill: none; }}
        .cut-label {{ fill: #333; font-family: Arial, sans-serif; font-size: 12px; font-weight: bold; }}
    </style>"#)?;
    
    // Draw edges first (so they appear under nodes)
    draw_edges(&mut svg, tree, &layouts)?;
    
    // Draw nodes
    draw_nodes(&mut svg, tree, &layouts)?;
    
    writeln!(&mut svg, "</svg>")?;
    
    Ok(svg)
}

/// Calculates layout positions for all nodes.
fn calculate_layouts(
    tree: &SlicingTree,
    node_radius: f64,
    min_spacing: f64,
) -> Vec<NodeLayout> {
    let mut layouts = vec![
        NodeLayout {
            x: 0.0,
            y: 0.0,
            subtree_width: 0.0,
        };
        tree.len()
    ];
    
    // Calculate subtree widths bottom-up
    calculate_subtree_width(tree, 0, &mut layouts, node_radius, min_spacing);
    
    // Position nodes top-down
    let root_width = layouts[0].subtree_width;
    layouts[0].x = root_width / 2.0 + node_radius;
    layouts[0].y = node_radius + 10.0;
    
    position_children(tree, 0, &mut layouts);
    
    layouts
}

/// Calculates the width needed for a subtree.
fn calculate_subtree_width(
    tree: &SlicingTree,
    idx: u16,
    layouts: &mut [NodeLayout],
    node_radius: f64,
    min_spacing: f64,
) -> f64 {
    let node = tree.node(idx);
    
    let width = match node {
        Node::Leaf { .. } => node_radius * 2.0,
        Node::Internal { left, right, .. } => {
            let left_width = calculate_subtree_width(tree, *left, layouts, node_radius, min_spacing);
            let right_width = calculate_subtree_width(tree, *right, layouts, node_radius, min_spacing);
            left_width + right_width + min_spacing
        }
    };
    
    layouts[idx as usize].subtree_width = width;
    width
}

/// Positions children recursively based on parent position.
fn position_children(
    tree: &SlicingTree,
    idx: u16,
    layouts: &mut [NodeLayout],
) {
    let node = tree.node(idx);
    
    if let Node::Internal { left, right, .. } = node {
        let parent_layout = layouts[idx as usize];
        let left_width = layouts[*left as usize].subtree_width;
        let right_width = layouts[*right as usize].subtree_width;
        
        // Position left child
        layouts[*left as usize].x = parent_layout.x - right_width / 2.0 - left_width / 2.0;
        layouts[*left as usize].y = parent_layout.y + 80.0;
        
        // Position right child
        layouts[*right as usize].x = parent_layout.x + left_width / 2.0 + right_width / 2.0;
        layouts[*right as usize].y = parent_layout.y + 80.0;
        
        // Recurse
        position_children(tree, *left, layouts);
        position_children(tree, *right, layouts);
    }
}

/// Draws edges between nodes.
fn draw_edges(
    svg: &mut String,
    tree: &SlicingTree,
    layouts: &[NodeLayout],
) -> fmt::Result {
    writeln!(svg, r#"<g class="edges">"#)?;
    
    for (idx, node) in tree.nodes().iter().enumerate() {
        if let Node::Internal { left, right, .. } = node {
            let parent = &layouts[idx];
            let left_child = &layouts[*left as usize];
            let right_child = &layouts[*right as usize];
            
            // Draw edge to left child
            writeln!(
                svg,
                r#"<line class="edge" x1="{}" y1="{}" x2="{}" y2="{}"/>"#,
                parent.x, parent.y, left_child.x, left_child.y
            )?;
            
            // Draw edge to right child
            writeln!(
                svg,
                r#"<line class="edge" x1="{}" y1="{}" x2="{}" y2="{}"/>"#,
                parent.x, parent.y, right_child.x, right_child.y
            )?;
        }
    }
    
    writeln!(svg, "</g>")?;
    Ok(())
}

/// Draws nodes with labels.
fn draw_nodes(
    svg: &mut String,
    tree: &SlicingTree,
    layouts: &[NodeLayout],
) -> fmt::Result {
    writeln!(svg, r#"<g class="nodes">"#)?;
    
    for (idx, node) in tree.nodes().iter().enumerate() {
        let layout = &layouts[idx];
        
        match node {
            Node::Leaf { photo_idx, .. } => {
                // Draw leaf node (green circle)
                writeln!(
                    svg,
                    r#"<circle class="leaf-node" cx="{}" cy="{}" r="{}"/>"#,
                    layout.x, layout.y, NODE_RADIUS
                )?;
                
                // Draw photo index
                writeln!(
                    svg,
                    r#"<text class="node-text" x="{}" y="{}">{}</text>"#,
                    layout.x, layout.y, photo_idx
                )?;
            }
            Node::Internal { cut, .. } => {
                // Draw internal node (blue circle)
                writeln!(
                    svg,
                    r#"<circle class="internal-node" cx="{}" cy="{}" r="{}"/>"#,
                    layout.x, layout.y, NODE_RADIUS
                )?;
                
                // Draw cut type
                let cut_str = match cut {
                    Cut::V => "V",
                    Cut::H => "H",
                };
                writeln!(
                    svg,
                    r#"<text class="node-text" x="{}" y="{}">{}</text>"#,
                    layout.x, layout.y, cut_str
                )?;
            }
        }
    }
    
    writeln!(svg, "</g>")?;
    Ok(())
}

/// Calculates the maximum depth of the tree.
fn calculate_depth(tree: &SlicingTree, idx: u16) -> usize {
    let node = tree.node(idx);
    
    match node {
        Node::Leaf { .. } => 1,
        Node::Internal { left, right, .. } => {
            let left_depth = calculate_depth(tree, *left);
            let right_depth = calculate_depth(tree, *right);
            1 + left_depth.max(right_depth)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visualize_single_photo() {
        let tree = SlicingTree::new(vec![Node::Leaf {
            photo_idx: 0,
            parent: None,
        }]);
        
        let svg = generate_svg(&tree).unwrap();
        assert!(svg.contains("svg"));
        assert!(svg.contains("leaf-node"));
        assert!(svg.contains(">0</text>"));
    }

    #[test]
    fn test_visualize_two_photos_v_cut() {
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
        
        let svg = generate_svg(&tree).unwrap();
        assert!(svg.contains("svg"));
        assert!(svg.contains("internal-node"));
        assert!(svg.contains("leaf-node"));
        assert!(svg.contains(">V</text>"));
        assert!(svg.contains(">0</text>"));
        assert!(svg.contains(">1</text>"));
    }

    #[test]
    fn test_visualize_file_creation() {
        let tree = SlicingTree::new(vec![
            Node::Internal {
                cut: Cut::H,
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
        ]);
        
        let temp_path = "/tmp/test_tree.svg";
        let result = visualize_tree(&tree, temp_path);
        assert!(result.is_ok());
        
        // Check file exists and contains expected content
        let content = std::fs::read_to_string(temp_path).unwrap();
        assert!(content.contains("svg"));
        assert!(content.contains(">H</text>"));
        
        // Cleanup
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_visualize_complex_tree() {
        // Create a more complex tree with 4 photos
        //        V(0)
        //       /    \
        //     H(1)   H(4)
        //    /  \    /  \
        //   P0  P1  P2  P3
        let tree = SlicingTree::new(vec![
            Node::Internal {
                cut: Cut::V,
                left: 1,
                right: 4,
                parent: None,
            },
            Node::Internal {
                cut: Cut::H,
                left: 2,
                right: 3,
                parent: Some(0),
            },
            Node::Leaf {
                photo_idx: 0,
                parent: Some(1),
            },
            Node::Leaf {
                photo_idx: 1,
                parent: Some(1),
            },
            Node::Internal {
                cut: Cut::H,
                left: 5,
                right: 6,
                parent: Some(0),
            },
            Node::Leaf {
                photo_idx: 2,
                parent: Some(4),
            },
            Node::Leaf {
                photo_idx: 3,
                parent: Some(4),
            },
        ]);
        
        let svg = generate_svg(&tree).unwrap();
        
        // Verify structure
        assert!(svg.contains("svg"));
        assert!(svg.contains(">V</text>")); // Root
        assert!(svg.contains(">H</text>")); // Two H-cuts
        assert!(svg.contains(">0</text>")); // Photo 0
        assert!(svg.contains(">1</text>")); // Photo 1
        assert!(svg.contains(">2</text>")); // Photo 2
        assert!(svg.contains(">3</text>")); // Photo 3
        
        // Count nodes (excluding CSS class definitions)
        let internal_count = svg.matches(r#"<circle class="internal-node""#).count();
        let leaf_count = svg.matches(r#"<circle class="leaf-node""#).count();
        assert_eq!(internal_count, 3); // 3 internal nodes
        assert_eq!(leaf_count, 4); // 4 leaf nodes
    }
}
