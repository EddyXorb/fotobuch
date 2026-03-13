//! Fitness function components for the genetic algorithm.

use super::super::data_models::{Canvas, Photo, SolverPageLayout};
use crate::dto_models::FitnessWeights;
/// Threshold ratio below which a photo is considered severely undersized.
const UNDERSIZED_THRESHOLD: f64 = 0.5;

/// Penalty multiplier for severely undersized photos.
const UNDERSIZED_PENALTY_MULTIPLIER: f64 = 50.0;

/// Breakdown of individual fitness cost components.
#[derive(Debug, Clone)]
pub struct CostBreakdown {
    pub size: f64,
    pub coverage: f64,
    pub barycenter: f64,
    pub total: f64,
}

/// Computes a detailed breakdown of all cost components, weighted as in the fitness function.
pub fn cost_breakdown(
    layout: &SolverPageLayout,
    photos: &[Photo],
    canvas: &Canvas,
    weights: &FitnessWeights,
) -> CostBreakdown {
    let size = weights.w_size * cost_size_distribution(layout, photos);
    let coverage = weights.w_coverage * cost_coverage(layout);
    let barycenter = weights.w_barycenter * cost_barycenter(layout);
    let total = total_cost(layout, photos, canvas, weights);
    CostBreakdown {
        size,
        coverage,
        barycenter,
        total,
    }
}

/// Computes the total cost of a layout using the given weights.
///
/// Skips terms with zero weight for efficiency.
pub fn total_cost(
    layout: &SolverPageLayout,
    photos: &[Photo],
    _canvas: &Canvas,
    weights: &FitnessWeights,
) -> f64 {
    let mut cost = 0.0;

    if weights.w_size != 0.0 {
        cost += weights.w_size * cost_size_distribution(layout, photos);
    }

    if weights.w_coverage != 0.0 {
        cost += weights.w_coverage * cost_coverage(layout);
    }

    if weights.w_barycenter != 0.0 {
        cost += weights.w_barycenter * cost_barycenter(layout);
    }

    cost
}

/// C1: Size distribution cost.
///
/// Penalizes deviation from target sizes, with extra penalty for undersized photos.
///
/// Formula: C1 = Σ k_i · (s_i - t_i)²
/// where:
/// - s_i = (w_i · h_i) / canvas_area (normalized actual size)
/// - t_i = area_weight_i / Σ area_weights (normalized target size)
/// - k_i = UNDERSIZED_PENALTY_MULTIPLIER if s_i/t_i < UNDERSIZED_THRESHOLD, else 1
fn cost_size_distribution(layout: &SolverPageLayout, photos: &[Photo]) -> f64 {
    let canvas_area = layout.canvas.area();

    // Compute normalized target sizes (sum = 1)
    let total_weight: f64 = photos.iter().map(|p| p.area_weight).sum();

    if total_weight == 0.0 {
        return 0.0;
    }

    let mut cost = 0.0;

    for placement in &layout.placements {
        let photo_idx = placement.photo_idx as usize;
        if photo_idx >= photos.len() {
            continue;
        }

        let photo = &photos[photo_idx];

        // Normalized actual size
        let s_i = placement.area() / canvas_area;

        // Normalized target size
        let t_i = photo.area_weight / total_weight;

        // Penalty coefficient: higher penalty for severely undersized photos
        let k_i = if s_i / t_i < UNDERSIZED_THRESHOLD {
            UNDERSIZED_PENALTY_MULTIPLIER
        } else {
            1.0
        };

        // Squared deviation
        let deviation = s_i - t_i;
        cost += k_i * deviation * deviation;
    }

    cost
}

/// C2: Canvas coverage cost.
///
/// Penalizes empty space on the canvas. Returns a value between 0 and 1,
/// where 0 means full coverage and 1 means empty canvas.
fn cost_coverage(layout: &SolverPageLayout) -> f64 {
    1.0 - layout.coverage_ratio()
}

/// C_bary: Barycenter centering cost.
///
/// Penalizes layouts where the area-weighted center is not at the canvas center.
fn cost_barycenter(layout: &SolverPageLayout) -> f64 {
    // Simple implementation for now
    let (bx, by) = layout.barycenter();
    let canvas = &layout.canvas;
    let dx = (bx - canvas.width / 2.0) / canvas.width;
    let dy = (by - canvas.height / 2.0) / canvas.height;
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::super::super::data_models::{Canvas, Photo, PhotoPlacement};
    use super::*;
    use approx::assert_relative_eq;

    fn make_photo(aspect_ratio: f64, area_weight: f64) -> Photo {
        let id = format!("test_{}", aspect_ratio);
        Photo::new(id, aspect_ratio, area_weight, "test".to_string())
    }

    #[test]
    fn test_total_cost_zero_weights() {
        let canvas = Canvas::default();
        let layout = SolverPageLayout::new(vec![], canvas);
        let photos = vec![];
        let weights = FitnessWeights::new(0.0, 0.0, 0.0, 0.0);

        let cost = total_cost(&layout, &photos, &canvas, &weights);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_cost_coverage_empty() {
        let canvas = Canvas::default();
        let layout = SolverPageLayout::new(vec![], canvas);
        let coverage_cost = cost_coverage(&layout);
        assert_eq!(coverage_cost, 1.0);
    }

    #[test]
    fn test_cost_coverage_full() {
        let canvas = Canvas::new(100.0, 100.0, 0.0);
        let placements = vec![PhotoPlacement::new(0, 0.0, 0.0, 100.0, 100.0)];
        let layout = SolverPageLayout::new(placements, canvas);

        let coverage_cost = cost_coverage(&layout);
        assert_relative_eq!(coverage_cost, 0.0, epsilon = 1e-6);
    }

    #[test]
    fn test_cost_coverage_half() {
        let canvas = Canvas::new(100.0, 100.0, 0.0);
        let placements = vec![PhotoPlacement::new(0, 0.0, 0.0, 50.0, 100.0)];
        let layout = SolverPageLayout::new(placements, canvas);

        let coverage_cost = cost_coverage(&layout);
        assert_relative_eq!(coverage_cost, 0.5, epsilon = 1e-6);
    }

    #[test]
    fn test_cost_barycenter_centered() {
        let canvas = Canvas::new(200.0, 200.0, 0.0);
        // Single photo centered at (100, 100)
        let placements = vec![PhotoPlacement::new(0, 50.0, 50.0, 100.0, 100.0)];
        let layout = SolverPageLayout::new(placements, canvas);

        let cost = cost_barycenter(&layout);
        assert_relative_eq!(cost, 0.0, epsilon = 1e-6);
    }

    #[test]
    fn test_cost_barycenter_offset() {
        let canvas = Canvas::new(200.0, 200.0, 0.0);
        // Photo in top-left corner, center at (25, 25)
        let placements = vec![PhotoPlacement::new(0, 0.0, 0.0, 50.0, 50.0)];
        let layout = SolverPageLayout::new(placements, canvas);

        let cost = cost_barycenter(&layout);

        // Barycenter at (25, 25), canvas center at (100, 100)
        // dx = (25 - 100) / 200 = -0.375, dy = -0.375
        // cost = 0.375^2 + 0.375^2 = 0.28125
        assert_relative_eq!(cost, 0.28125, epsilon = 1e-6);
    }

    #[test]
    fn test_cost_size_distribution_perfect() {
        let canvas = Canvas::new(200.0, 100.0, 0.0); // 20000 mm²
        let photos = vec![
            make_photo(1.0, 1.0), // Weight 1/3, target 6666.67 mm²
            make_photo(1.0, 1.0), // Weight 1/3
            make_photo(1.0, 1.0), // Weight 1/3
        ];

        // Each photo gets exactly 1/3 of canvas area
        let placements = vec![
            PhotoPlacement::new(0, 0.0, 0.0, 100.0, 66.67), // ~6667 mm²
            PhotoPlacement::new(1, 0.0, 66.67, 100.0, 66.67), // ~6667 mm²
            PhotoPlacement::new(2, 100.0, 0.0, 100.0, 100.0), // ~10000 mm² (will have deviation)
        ];

        let layout = SolverPageLayout::new(placements, canvas);
        let cost = cost_size_distribution(&layout, &photos);

        // There will be some cost due to imperfect division
        assert!(cost >= 0.0);
    }

    #[test]
    fn test_cost_size_distribution_undersized_penalty() {
        let canvas = Canvas::new(100.0, 100.0, 0.0); // 10000 mm²
        let photos = vec![
            make_photo(1.0, 1.0), // Weight 1, target 100% = 10000 mm²
        ];

        // Photo gets only 40% of target (2000 vs 10000)
        // s_i = 2000/10000 = 0.2
        // t_i = 1.0
        // s_i/t_i = 0.2 < 0.5 → k_i = 50
        // cost = 50 * (0.2 - 1.0)² = 50 * 0.64 = 32.0
        let placements = vec![PhotoPlacement::new(0, 0.0, 0.0, 20.0, 100.0)];
        let layout = SolverPageLayout::new(placements, canvas);

        let cost = cost_size_distribution(&layout, &photos);
        assert_relative_eq!(cost, 32.0, epsilon = 1e-6);
    }

    #[test]
    fn test_cost_size_distribution_normal_penalty() {
        let canvas = Canvas::new(100.0, 100.0, 0.0); // 10000 mm²
        let photos = vec![
            make_photo(1.0, 1.0), // Weight 1, target 100% = 10000 mm²
        ];

        // Photo gets 80% of target (8000 vs 10000)
        // s_i = 8000/10000 = 0.8
        // t_i = 1.0
        // s_i/t_i = 0.8 >= 0.5 → k_i = 1
        // cost = 1 * (0.8 - 1.0)² = 0.04
        let placements = vec![PhotoPlacement::new(0, 0.0, 0.0, 80.0, 100.0)];
        let layout = SolverPageLayout::new(placements, canvas);

        let cost = cost_size_distribution(&layout, &photos);
        assert_relative_eq!(cost, 0.04, epsilon = 1e-6);
    }

    #[test]
    fn test_cost_reading_order_correct() {
        let canvas = Canvas::new(200.0, 100.0, 0.0);
        // Photos in correct reading order: left to right, top to bottom
        let placements = vec![
            PhotoPlacement::new(0, 0.0, 0.0, 50.0, 50.0), // Top-left
            PhotoPlacement::new(1, 50.0, 0.0, 50.0, 50.0), // Top-right
            PhotoPlacement::new(2, 0.0, 50.0, 50.0, 50.0), // Bottom-left
        ];
        let layout = SolverPageLayout::new(placements, canvas);
        let photos = vec![
            make_photo(1.0, 1.0),
            make_photo(1.0, 1.0),
            make_photo(1.0, 1.0),
        ];

        let cost = cost_reading_order(&layout, &photos);
        assert_relative_eq!(cost, 0.0, epsilon = 1e-6);
    }

    #[test]
    fn test_cost_reading_order_inverted() {
        let canvas = Canvas::new(200.0, 100.0, 0.0);
        // Photos in wrong order: later photos appear earlier in reading
        let placements = vec![
            PhotoPlacement::new(0, 100.0, 50.0, 50.0, 50.0), // Photo 0 bottom-right
            PhotoPlacement::new(1, 0.0, 0.0, 50.0, 50.0),    // Photo 1 top-left
        ];
        let layout = SolverPageLayout::new(placements, canvas);
        let photos = vec![make_photo(1.0, 1.0), make_photo(1.0, 1.0)];

        // Photo 0 score: 100/200 + 50/100 = 0.5 + 0.5 = 1.0
        // Photo 1 score: 0/200 + 0/100 = 0.0
        // Inversion: max(0, 1.0 - 0.0) = 1.0
        let cost = cost_reading_order(&layout, &photos);
        assert_relative_eq!(cost, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_cost_reading_order_single_photo() {
        let canvas = Canvas::new(100.0, 100.0, 0.0);
        let placements = vec![PhotoPlacement::new(0, 0.0, 0.0, 50.0, 50.0)];
        let layout = SolverPageLayout::new(placements, canvas);
        let photos = vec![make_photo(1.0, 1.0)];

        let cost = cost_reading_order(&layout, &photos);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_total_cost_combined() {
        let canvas = Canvas::new(100.0, 100.0, 0.0);
        let photos = vec![make_photo(1.0, 1.0)];
        let placements = vec![PhotoPlacement::new(0, 0.0, 0.0, 50.0, 50.0)];
        let layout = SolverPageLayout::new(placements, canvas);

        let weights = FitnessWeights::default();
        let cost = total_cost(&layout, &photos, &canvas, &weights);

        // Should be sum of all weighted components
        assert!(cost > 0.0);
    }
}
