//! Conversion layer between solver output and I/O formats.
//!
//! This module provides transformations for layout results, such as:
//! - Centering layouts on the canvas
//! - Adding margins or offsets
//! - Coordinate system transformations

use crate::model::{Canvas, LayoutResult, PhotoPlacement};

/// Centers a layout result on its canvas by adding offsets to all placements.
///
/// The solver produces layouts that start at (0, 0). This function calculates
/// the bounding box of all placements and adds offsets to center the layout
/// on the canvas.
///
/// # Arguments
///
/// * `layout` - The layout result from the solver
///
/// # Returns
///
/// A new `LayoutResult` with centered placements.
pub fn center_layout(layout: &LayoutResult) -> LayoutResult {
    if layout.placements.is_empty() {
        return layout.clone();
    }

    // Calculate bounding box of the layout
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for p in &layout.placements {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x + p.w);
        max_y = max_y.max(p.y + p.h);
    }

    let layout_width = max_x - min_x;
    let layout_height = max_y - min_y;

    // Calculate centering offsets
    let offset_x = (layout.canvas.width - layout_width) / 2.0 - min_x;
    let offset_y = (layout.canvas.height - layout_height) / 2.0 - min_y;

    // Apply offsets to all placements
    let centered_placements: Vec<PhotoPlacement> = layout
        .placements
        .iter()
        .map(|p| {
            PhotoPlacement::new(
                p.photo_idx,
                p.x + offset_x,
                p.y + offset_y,
                p.w,
                p.h,
            )
        })
        .collect();

    LayoutResult::new(centered_placements, layout.canvas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_center_layout_already_centered() {
        let canvas = Canvas::new(200.0, 200.0, 2.0, 0.0);
        let placements = vec![
            PhotoPlacement::new(0, 50.0, 50.0, 100.0, 100.0),
        ];
        let layout = LayoutResult::new(placements, canvas);

        let centered = center_layout(&layout);

        // Layout is 100x100 at (50, 50), so it's already centered
        let p = &centered.placements[0];
        assert_relative_eq!(p.x, 50.0, epsilon = 1e-6);
        assert_relative_eq!(p.y, 50.0, epsilon = 1e-6);
    }

    #[test]
    fn test_center_layout_offset() {
        let canvas = Canvas::new(500.0, 500.0, 2.0, 0.0);
        // Layout starts at (0,0) with dimensions 100x100
        let placements = vec![
            PhotoPlacement::new(0, 0.0, 0.0, 100.0, 100.0),
        ];
        let layout = LayoutResult::new(placements, canvas);

        let centered = center_layout(&layout);

        // Should be centered: offset = (500 - 100) / 2 = 200
        let p = &centered.placements[0];
        assert_relative_eq!(p.x, 200.0, epsilon = 1e-6);
        assert_relative_eq!(p.y, 200.0, epsilon = 1e-6);
        assert_relative_eq!(p.w, 100.0, epsilon = 1e-6);
        assert_relative_eq!(p.h, 100.0, epsilon = 1e-6);
    }

    #[test]
    fn test_center_layout_multiple_placements() {
        let canvas = Canvas::new(300.0, 300.0, 0.0, 0.0);
        // Two photos side by side: 0-100 and 100-200, total 200x100
        let placements = vec![
            PhotoPlacement::new(0, 0.0, 0.0, 100.0, 100.0),
            PhotoPlacement::new(1, 100.0, 0.0, 100.0, 100.0),
        ];
        let layout = LayoutResult::new(placements, canvas);

        let centered = center_layout(&layout);

        // Layout is 200x100, canvas is 300x300
        // offset_x = (300 - 200) / 2 = 50
        // offset_y = (300 - 100) / 2 = 100
        assert_relative_eq!(centered.placements[0].x, 50.0, epsilon = 1e-6);
        assert_relative_eq!(centered.placements[0].y, 100.0, epsilon = 1e-6);
        assert_relative_eq!(centered.placements[1].x, 150.0, epsilon = 1e-6);
        assert_relative_eq!(centered.placements[1].y, 100.0, epsilon = 1e-6);
    }

    #[test]
    fn test_center_layout_with_negative_coords() {
        let canvas = Canvas::new(400.0, 400.0, 0.0, 0.0);
        // Layout has some negative coordinates (shouldn't happen in practice, but test it)
        let placements = vec![
            PhotoPlacement::new(0, -10.0, -10.0, 100.0, 100.0),
            PhotoPlacement::new(1, 90.0, 90.0, 100.0, 100.0),
        ];
        let layout = LayoutResult::new(placements, canvas);

        let centered = center_layout(&layout);

        // Bounding box: (-10, -10) to (190, 190) = 200x200
        // offset_x = (400 - 200) / 2 - (-10) = 100 + 10 = 110
        // offset_y = (400 - 200) / 2 - (-10) = 100 + 10 = 110
        assert_relative_eq!(centered.placements[0].x, 100.0, epsilon = 1e-6);
        assert_relative_eq!(centered.placements[0].y, 100.0, epsilon = 1e-6);
        assert_relative_eq!(centered.placements[1].x, 200.0, epsilon = 1e-6);
        assert_relative_eq!(centered.placements[1].y, 200.0, epsilon = 1e-6);
    }

    #[test]
    fn test_center_layout_empty() {
        let canvas = Canvas::new(200.0, 200.0, 2.0, 0.0);
        let layout = LayoutResult::new(vec![], canvas);

        let centered = center_layout(&layout);

        assert!(centered.placements.is_empty());
    }

    #[test]
    fn test_center_layout_tall_canvas() {
        let canvas = Canvas::new(200.0, 400.0, 2.0, 0.0);
        // Layout uses full width but only part of height
        let placements = vec![
            PhotoPlacement::new(0, 0.0, 0.0, 200.0, 100.0),
        ];
        let layout = LayoutResult::new(placements, canvas);

        let centered = center_layout(&layout);

        // offset_x = (200 - 200) / 2 = 0
        // offset_y = (400 - 100) / 2 = 150
        let p = &centered.placements[0];
        assert_relative_eq!(p.x, 0.0, epsilon = 1e-6);
        assert_relative_eq!(p.y, 150.0, epsilon = 1e-6);
    }
}
