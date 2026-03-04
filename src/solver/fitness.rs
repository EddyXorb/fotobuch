//! Fitness function components for the genetic algorithm.

use crate::model::{Canvas, FitnessWeights, LayoutResult, Photo};

/// Computes the total cost of a layout using the given weights.
///
/// Skips terms with zero weight for efficiency.
pub fn total_cost(
    layout: &LayoutResult,
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

    if weights.w_order != 0.0 {
        cost += weights.w_order * cost_reading_order(layout, photos);
    }

    cost
}

/// C1: Size distribution cost.
///
/// Penalizes deviation from target sizes, with extra penalty for undersized photos.
fn cost_size_distribution(_layout: &LayoutResult, _photos: &[Photo]) -> f64 {
    // TODO: Implement in Step 4
    0.0
}

/// C2: Canvas coverage cost.
///
/// Penalizes empty space on the canvas.
fn cost_coverage(layout: &LayoutResult) -> f64 {
    // TODO: Implement in Step 4
    1.0 - layout.coverage_ratio()
}

/// C_bary: Barycenter centering cost.
///
/// Penalizes layouts where the area-weighted center is not at the canvas center.
fn cost_barycenter(layout: &LayoutResult) -> f64 {
    // Simple implementation for now
    let (bx, by) = layout.barycenter();
    let canvas = &layout.canvas;
    let dx = (bx - canvas.width / 2.0) / canvas.width;
    let dy = (by - canvas.height / 2.0) / canvas.height;
    dx * dx + dy * dy
}

/// C_order: Reading order cost.
///
/// Penalizes inversions in the chronological reading order.
fn cost_reading_order(_layout: &LayoutResult, _photos: &[Photo]) -> f64 {
    // TODO: Implement in Step 4
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Canvas;

    #[test]
    fn test_total_cost_zero_weights() {
        let canvas = Canvas::default();
        let layout = LayoutResult::new(vec![], canvas);
        let photos = vec![];
        let weights = FitnessWeights::new(0.0, 0.0, 0.0, 0.0);

        let cost = total_cost(&layout, &photos, &canvas, &weights);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_cost_coverage_empty() {
        let canvas = Canvas::default();
        let layout = LayoutResult::new(vec![], canvas);
        let coverage_cost = cost_coverage(&layout);
        assert_eq!(coverage_cost, 1.0);
    }
}
