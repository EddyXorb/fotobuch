//! Affine layout solver for slicing trees with β (gap) support.
//!
//! Implements the O(N) bottom-up/top-down algorithm with affine coefficients.
//! Each node stores (α, γ) such that: w = α·h + γ
//! This allows handling β > 0 without falling back to solving linear systems.

use crate::models::{Canvas, LayoutResult, Photo, PhotoPlacement};
use super::tree::{Cut, Node, SlicingTree};

/// Affine coefficient pair (α, γ) representing the relationship w = α·h + γ.
#[derive(Debug, Clone, Copy)]
struct AffineCoeff {
    alpha: f64,
    gamma: f64,
}

impl AffineCoeff {
    fn new(alpha: f64, gamma: f64) -> Self {
        Self { alpha, gamma }
    }
}

/// Dimensions (width, height) for a node.
#[derive(Debug, Clone, Copy)]
struct Dimensions {
    w: f64,
    h: f64,
}

/// Position (x, y) for a node.
#[derive(Debug, Clone, Copy)]
struct Position {
    x: f64,
    y: f64,
}

/// Solves the layout for a slicing tree with given photos and canvas.
///
/// Algorithm:
/// 1. Compute affine coefficients bottom-up
/// 2. Assign dimensions top-down from root
/// 3. Compute positions top-down from root
///
/// Returns a LayoutResult with all photo placements.
pub fn solve_layout(
    tree: &SlicingTree,
    photos: &[Photo],
    canvas: &Canvas,
) -> LayoutResult {
    if tree.is_empty() {
        return LayoutResult::new(vec![], *canvas);
    }

    // Step 1: Compute affine coefficients for all nodes (bottom-up)
    let coeffs = compute_coefficients(tree, photos, canvas.beta);

    // Step 2: Assign dimensions to all nodes (top-down)
    let dims = compute_dimensions(tree, &coeffs, canvas);

    // Step 3: Compute positions for all nodes (top-down)
    let positions = compute_positions(tree, &dims, canvas.beta);

    // Extract placements for leaf nodes
    let mut placements = Vec::new();
    for (idx, node) in tree.nodes().iter().enumerate() {
        if let Node::Leaf { photo_idx, .. } = node {
            let dim = dims[idx];
            let pos = positions[idx];
            placements.push(PhotoPlacement::new(
                *photo_idx,
                pos.x,
                pos.y,
                dim.w,
                dim.h,
            ));
        }
    }

    LayoutResult::new(placements, *canvas)
}

/// Computes affine coefficients for all nodes (bottom-up).
///
/// For each node, computes (α, γ) such that w = α·h + γ.
fn compute_coefficients(
    tree: &SlicingTree,
    photos: &[Photo],
    beta: f64,
) -> Vec<AffineCoeff> {
    let mut coeffs = vec![AffineCoeff::new(0.0, 0.0); tree.len()];
    compute_coefficients_recursive(tree, photos, beta, 0, &mut coeffs);
    coeffs
}

fn compute_coefficients_recursive(
    tree: &SlicingTree,
    photos: &[Photo],
    beta: f64,
    idx: u16,
    coeffs: &mut [AffineCoeff],
) -> AffineCoeff {
    match tree.node(idx) {
        Node::Leaf { photo_idx, .. } => {
            // Leaf: w = a·h + 0
            let alpha = photos[*photo_idx as usize].aspect_ratio;
            let coeff = AffineCoeff::new(alpha, 0.0);
            coeffs[idx as usize] = coeff;
            coeff
        }
        Node::Internal { cut, left, right, .. } => {
            // Recursively compute children first
            let coeff_l = compute_coefficients_recursive(tree, photos, beta, *left, coeffs);
            let coeff_r = compute_coefficients_recursive(tree, photos, beta, *right, coeffs);

            let coeff = match cut {
                Cut::V => {
                    // V-node: children side by side (same height)
                    // w = w_l + w_r + β
                    //   = (α_l·h + γ_l) + (α_r·h + γ_r) + β
                    //   = (α_l + α_r)·h + (γ_l + γ_r + β)
                    AffineCoeff::new(
                        coeff_l.alpha + coeff_r.alpha,
                        coeff_l.gamma + coeff_r.gamma + beta,
                    )
                }
                Cut::H => {
                    // H-node: children stacked (same width)
                    // From w = α·h + γ → h = (w - γ)/α
                    // h = h_l + h_r + β
                    //   = (w - γ_l)/α_l + (w - γ_r)/α_r + β
                    //   = w·(1/α_l + 1/α_r) + (-γ_l/α_l - γ_r/α_r + β)
                    // 
                    // Rearranging to w = α·h + γ form:
                    // Let S = 1/α_l + 1/α_r
                    // h = w·S + (-γ_l/α_l - γ_r/α_r + β)
                    // w = h/S - (-γ_l/α_l - γ_r/α_r + β)/S
                    // w = (α_l·α_r/(α_l + α_r))·h + (γ_l/α_l + γ_r/α_r - β)·(α_l·α_r/(α_l + α_r))

                    let s = 1.0 / coeff_l.alpha + 1.0 / coeff_r.alpha;
                    let alpha = 1.0 / s; // = α_l·α_r/(α_l + α_r)
                    let gamma = (coeff_l.gamma / coeff_l.alpha 
                               + coeff_r.gamma / coeff_r.alpha 
                               - beta) * alpha;
                    AffineCoeff::new(alpha, gamma)
                }
            };

            coeffs[idx as usize] = coeff;
            coeff
        }
    }
}

/// Assigns dimensions to all nodes (top-down).
fn compute_dimensions(
    tree: &SlicingTree,
    coeffs: &[AffineCoeff],
    canvas: &Canvas,
) -> Vec<Dimensions> {
    let mut dims = vec![Dimensions { w: 0.0, h: 0.0 }; tree.len()];

    // Root: determine dimensions based on canvas
    let root_coeff = coeffs[0];
    
    // Try filling height first
    let h_try = canvas.height;
    let w_try = root_coeff.alpha * h_try + root_coeff.gamma;

    let (root_w, root_h) = if w_try <= canvas.width {
        (w_try, h_try)
    } else {
        // Width is the limiting factor
        let w = canvas.width;
        let h = (w - root_coeff.gamma) / root_coeff.alpha;
        (w, h)
    };

    dims[0] = Dimensions { w: root_w, h: root_h };

    // Recursively assign dimensions to children
    compute_dimensions_recursive(tree, coeffs, 0, &mut dims);

    dims
}

fn compute_dimensions_recursive(
    tree: &SlicingTree,
    coeffs: &[AffineCoeff],
    idx: u16,
    dims: &mut [Dimensions],
) {
    let node = tree.node(idx);
    let dim = dims[idx as usize];

    if let Node::Internal { cut, left, right, .. } = node {
        match cut {
            Cut::V => {
                // V-node: children have same height
                let h = dim.h;
                let coeff_l = coeffs[*left as usize];
                let coeff_r = coeffs[*right as usize];

                dims[*left as usize] = Dimensions {
                    w: coeff_l.alpha * h + coeff_l.gamma,
                    h,
                };
                dims[*right as usize] = Dimensions {
                    w: coeff_r.alpha * h + coeff_r.gamma,
                    h,
                };
            }
            Cut::H => {
                // H-node: children have same width
                let w = dim.w;
                let coeff_l = coeffs[*left as usize];
                let coeff_r = coeffs[*right as usize];

                dims[*left as usize] = Dimensions {
                    w,
                    h: (w - coeff_l.gamma) / coeff_l.alpha,
                };
                dims[*right as usize] = Dimensions {
                    w,
                    h: (w - coeff_r.gamma) / coeff_r.alpha,
                };
            }
        }

        // Recurse
        compute_dimensions_recursive(tree, coeffs, *left, dims);
        compute_dimensions_recursive(tree, coeffs, *right, dims);
    }
}

/// Computes positions for all nodes (top-down).
fn compute_positions(
    tree: &SlicingTree,
    dims: &[Dimensions],
    beta: f64,
) -> Vec<Position> {
    let mut positions = vec![Position { x: 0.0, y: 0.0 }; tree.len()];

    // Root starts at origin
    positions[0] = Position { x: 0.0, y: 0.0 };

    // Recursively assign positions to children
    compute_positions_recursive(tree, dims, beta, 0, &mut positions);

    positions
}

fn compute_positions_recursive(
    tree: &SlicingTree,
    dims: &[Dimensions],
    beta: f64,
    idx: u16,
    positions: &mut [Position],
) {
    let node = tree.node(idx);
    let pos = positions[idx as usize];

    if let Node::Internal { cut, left, right, .. } = node {
        let dim_l = dims[*left as usize];

        match cut {
            Cut::V => {
                // V-node: left child at same position, right child to the right with gap
                positions[*left as usize] = pos;
                positions[*right as usize] = Position {
                    x: pos.x + dim_l.w + beta,
                    y: pos.y,
                };
            }
            Cut::H => {
                // H-node: top child at same position, bottom child below with gap
                positions[*left as usize] = pos;
                positions[*right as usize] = Position {
                    x: pos.x,
                    y: pos.y + dim_l.h + beta,
                };
            }
        }

        // Recurse
        compute_positions_recursive(tree, dims, beta, *left, positions);
        compute_positions_recursive(tree, dims, beta, *right, positions);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::page_layout::tree::build::random_tree;
    use crate::solver::page_layout::tree::validate::validate_tree;
    use approx::assert_relative_eq;
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn make_photo(aspect_ratio: f64) -> Photo {
        Photo::new(aspect_ratio, 1.0, "test".to_string())
    }

    #[test]
    fn test_solve_layout_empty_tree() {
        let tree = SlicingTree::new(vec![Node::Leaf {
            photo_idx: 0,
            parent: None,
        }]);
        let photos = vec![make_photo(1.5)];
        let canvas = Canvas::new(300.0, 200.0, 0.0, 0.0);

        let layout = solve_layout(&tree, &photos, &canvas);
        assert_eq!(layout.placements.len(), 1);
        
        let p = &layout.placements[0];
        assert_eq!(p.photo_idx, 0);
        assert_relative_eq!(p.x, 0.0, epsilon = 1e-6);
        assert_relative_eq!(p.y, 0.0, epsilon = 1e-6);
        
        // Photo should fill height, width = 1.5 * 200 = 300
        assert_relative_eq!(p.h, 200.0, epsilon = 1e-6);
        assert_relative_eq!(p.w, 300.0, epsilon = 1e-6);
    }

    #[test]
    fn test_solve_layout_two_photos_v_no_beta() {
        // Two photos side by side, no gap
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
        let photos = vec![make_photo(1.5), make_photo(2.0)]; // total aspect: 3.5
        let canvas = Canvas::new(350.0, 100.0, 0.0, 0.0);

        let layout = solve_layout(&tree, &photos, &canvas);
        assert_eq!(layout.placements.len(), 2);

        // Both photos should have height = 100
        // Photo 0: w = 1.5 * 100 = 150
        // Photo 1: w = 2.0 * 100 = 200
        let p0 = layout.placements.iter().find(|p| p.photo_idx == 0).unwrap();
        let p1 = layout.placements.iter().find(|p| p.photo_idx == 1).unwrap();

        assert_relative_eq!(p0.h, 100.0, epsilon = 1e-6);
        assert_relative_eq!(p0.w, 150.0, epsilon = 1e-6);
        assert_relative_eq!(p0.x, 0.0, epsilon = 1e-6);
        assert_relative_eq!(p0.y, 0.0, epsilon = 1e-6);

        assert_relative_eq!(p1.h, 100.0, epsilon = 1e-6);
        assert_relative_eq!(p1.w, 200.0, epsilon = 1e-6);
        assert_relative_eq!(p1.x, 150.0, epsilon = 1e-6);
        assert_relative_eq!(p1.y, 0.0, epsilon = 1e-6);
    }

    #[test]
    fn test_solve_layout_two_photos_h_no_beta() {
        // Two photos stacked, no gap
        let nodes = vec![
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
        ];
        let tree = SlicingTree::new(nodes);
        let photos = vec![make_photo(2.0), make_photo(3.0)];
        let canvas = Canvas::new(300.0, 200.0, 0.0, 0.0);

        let layout = solve_layout(&tree, &photos, &canvas);
        assert_eq!(layout.placements.len(), 2);

        // Both photos should have width = 300
        // Photo 0: h = 300 / 2.0 = 150
        // Photo 1: h = 300 / 3.0 = 100
        // Total height = 250, but canvas is only 200, so it should scale down
        
        // Width is limiting: use full 300
        // 1/a_combined = 1/2 + 1/3 = 5/6, so a_combined = 6/5 = 1.2
        // h_total = w / a = 300 / 1.2 = 250
        // But canvas height is 200, so scale down: w = 200 * 1.2 = 240

        let p0 = layout.placements.iter().find(|p| p.photo_idx == 0).unwrap();
        let p1 = layout.placements.iter().find(|p| p.photo_idx == 1).unwrap();

        // Layout starts at origin (centering is done in converter layer)
        assert_relative_eq!(p0.w, 240.0, epsilon = 1e-6);
        assert_relative_eq!(p0.h, 120.0, epsilon = 1e-6);
        assert_relative_eq!(p0.x, 0.0, epsilon = 1e-6);
        assert_relative_eq!(p0.y, 0.0, epsilon = 1e-6);

        assert_relative_eq!(p1.w, 240.0, epsilon = 1e-6);
        assert_relative_eq!(p1.h, 80.0, epsilon = 1e-6);
        assert_relative_eq!(p1.x, 0.0, epsilon = 1e-6);
        assert_relative_eq!(p1.y, 120.0, epsilon = 1e-6);
    }

    #[test]
    fn test_solve_layout_with_beta() {
        // Two photos side by side with gap
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
        let photos = vec![make_photo(1.0), make_photo(1.0)]; // Square photos
        let canvas = Canvas::new(210.0, 100.0, 10.0, 0.0); // 10mm gap

        let layout = solve_layout(&tree, &photos, &canvas);
        assert_eq!(layout.placements.len(), 2);

        // w_total = w1 + w2 + beta = 100 + 100 + 10 = 210 (fits exactly)
        let p0 = layout.placements.iter().find(|p| p.photo_idx == 0).unwrap();
        let p1 = layout.placements.iter().find(|p| p.photo_idx == 1).unwrap();

        assert_relative_eq!(p0.w, 100.0, epsilon = 1e-6);
        assert_relative_eq!(p0.h, 100.0, epsilon = 1e-6);
        assert_relative_eq!(p0.x, 0.0, epsilon = 1e-6);

        assert_relative_eq!(p1.w, 100.0, epsilon = 1e-6);
        assert_relative_eq!(p1.h, 100.0, epsilon = 1e-6);
        assert_relative_eq!(p1.x, 110.0, epsilon = 1e-6); // 100 + 10 gap
    }

    #[test]
    fn test_solve_layout_random_trees() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        for n in 2..=10 {
            let tree = random_tree(n, &mut rng);
            assert!(validate_tree(&tree).is_ok());

            let photos: Vec<Photo> = (0..n)
                .map(|_| {
                    let ar = rng.gen_range(0.5..2.0);
                    make_photo(ar)
                })
                .collect();
            
            let canvas = Canvas::new(297.0, 210.0, 2.0, 0.0);
            let layout = solve_layout(&tree, &photos, &canvas);

            // Basic sanity checks
            assert_eq!(layout.placements.len(), n);
            
            // All photos should be within canvas bounds
            for p in &layout.placements {
                assert!(p.w > 0.0 && p.h > 0.0);
                assert!(p.x >= 0.0 && p.y >= 0.0);
                assert!(p.x + p.w <= canvas.width + 1e-6);
                assert!(p.y + p.h <= canvas.height + 1e-6);
                
                // Aspect ratio should match photo
                let expected_ar = photos[p.photo_idx as usize].aspect_ratio;
                let actual_ar = p.w / p.h;
                assert_relative_eq!(actual_ar, expected_ar, epsilon = 1e-6);
            }
        }
    }
}
