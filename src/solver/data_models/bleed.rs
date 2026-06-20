//! Bleed-aware output transforms for page layouts.
//!
//! This module extends [`SolverPageLayout`] with scaling operations that ensure
//! the layout respects print bleed requirements.

use super::layout::{PhotoPlacement, SolverPageLayout};
use crate::dto_models::CanvasConfig;

impl SolverPageLayout {
    fn scale_around_fixpoint(&self, factor: f64, fixpoint_x: f64, fixpoint_y: f64) -> Self {
        let scaled_placements: Vec<PhotoPlacement> = self
            .placements
            .iter()
            .map(|p| {
                let new_x = fixpoint_x + (p.x - fixpoint_x) * factor;
                let new_y = fixpoint_y + (p.y - fixpoint_y) * factor;
                PhotoPlacement::new(p.photo_idx, new_x, new_y, p.w * factor, p.h * factor)
            })
            .collect();

        SolverPageLayout::new(scaled_placements, self.canvas)
    }

    fn calc_needed_scaling_around_center_for_bleed(&self, book_config: &impl CanvasConfig) -> f64 {
        if book_config.margin_mm() > 0.0 || book_config.bleed_mm() == 0.0 {
            return 1.0;
        }
        let mut bleed_scale_factor = 1.0;
        let mut scale_factor_increase_last_iteration = 1.0;
        let mut bb = self.bounding_box();
        let (center_width, center_height) = self.canvas.center();

        loop {
            bb[0] = center_width + (bb[0] - center_width) * scale_factor_increase_last_iteration;
            bb[1] = center_height + (bb[1] - center_height) * scale_factor_increase_last_iteration;
            bb[2] = center_width + (bb[2] - center_width) * scale_factor_increase_last_iteration;
            bb[3] = center_height + (bb[3] - center_height) * scale_factor_increase_last_iteration;

            let border_distances = [
                bb[0],                      // left
                bb[1],                      // top
                self.canvas.width - bb[2],  // right
                self.canvas.height - bb[3], // bottom
            ];

            let needed_increase = border_distances
                .iter()
                .enumerate()
                .filter(|&(_, d)| {
                    d <= &book_config.bleed_threshold_mm() && d >= &-book_config.bleed_mm()
                })
                .map(|(i, d)| (i, f64::abs(-book_config.bleed_mm() - d)))
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            if needed_increase.is_none() || needed_increase.unwrap().1 <= 0.001 {
                break;
            }
            let idx_with_max = needed_increase.unwrap().0;

            if idx_with_max % 2 == 0 {
                let distance_to_center = f64::abs(center_width - bb[idx_with_max]);
                scale_factor_increase_last_iteration =
                    (distance_to_center + needed_increase.unwrap().1) / distance_to_center;
                bleed_scale_factor *= scale_factor_increase_last_iteration;
            } else {
                let distance_to_center = f64::abs(center_height - bb[idx_with_max]);
                scale_factor_increase_last_iteration =
                    (distance_to_center + needed_increase.unwrap().1) / distance_to_center;
                bleed_scale_factor *= scale_factor_increase_last_iteration;
            }
        }

        bleed_scale_factor
    }

    /// Zooms the layout in around the center to respect print bleed requirements.
    pub(crate) fn zoom_to_respect_bleed(&self, book_config: &impl CanvasConfig) -> Self {
        let scale_factor = self.calc_needed_scaling_around_center_for_bleed(book_config);
        let (center_x, center_y) = self.canvas.center();
        self.scale_around_fixpoint(scale_factor, center_x, center_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::BookConfig;
    use crate::solver::data_models::canvas::Canvas;
    use approx::assert_relative_eq;

    #[test]
    fn test_calc_scaling_for_bleed_no_bleed_due_to_margin() {
        let canvas = Canvas::new(200.0, 200.0, 0.0);
        let placements = vec![PhotoPlacement::new(0, 50.0, 50.0, 100.0, 100.0)];
        let layout = SolverPageLayout::new(placements, canvas);

        let book_config = BookConfig {
            margin_mm: 10.0,
            bleed_mm: 5.0,
            bleed_threshold_mm: 5.0,
            ..Default::default()
        };

        let scale_factor = layout.calc_needed_scaling_around_center_for_bleed(&book_config);
        assert_relative_eq!(scale_factor, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_calc_scaling_for_bleed_no_bleed_due_distance_to_print_border() {
        let canvas = Canvas::new(200.0, 200.0, 0.0);
        let placements = vec![PhotoPlacement::new(0, 5.0, 5.0, 100.0, 100.0)];
        let layout = SolverPageLayout::new(placements, canvas);

        let book_config = BookConfig {
            margin_mm: 0.0,
            bleed_mm: 5.0,
            bleed_threshold_mm: 4.99999,
            ..Default::default()
        };

        let scale_factor = layout.calc_needed_scaling_around_center_for_bleed(&book_config);
        assert_relative_eq!(scale_factor, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_calc_scaling_for_bleed_bleed_due_distance_to_print_border() {
        let canvas = Canvas::new(200.0, 200.0, 0.0);
        let placements = vec![PhotoPlacement::new(0, 5.0, 5.0, 100.0, 100.0)];
        let layout = SolverPageLayout::new(placements, canvas);

        let book_config = BookConfig {
            margin_mm: 0.0,
            bleed_mm: 5.0,
            bleed_threshold_mm: 5.0,
            ..Default::default()
        };

        let scale_factor = layout.calc_needed_scaling_around_center_for_bleed(&book_config);
        assert_relative_eq!(scale_factor, 105.0 / 95.0, epsilon = 1e-6);
    }

    #[test]
    fn test_calc_scaling_for_bleed_scales_correctly_height() {
        let canvas = Canvas::new(200.0, 200.0, 0.0);
        let placements = vec![PhotoPlacement::new(0, 100.0, 100.0, 10.0, 95.0)];
        let layout = SolverPageLayout::new(placements, canvas);

        let book_config = BookConfig {
            margin_mm: 0.0,
            bleed_mm: 5.0,
            bleed_threshold_mm: 5.0,
            ..Default::default()
        };

        let scale_factor = layout.calc_needed_scaling_around_center_for_bleed(&book_config);
        assert_relative_eq!(scale_factor, 105.0 / 95.0, epsilon = 1e-6);
    }

    #[test]
    fn test_calc_scaling_for_bleed_scales_correctly_width() {
        let canvas = Canvas::new(200.0, 200.0, 0.0);
        let placements = vec![PhotoPlacement::new(0, 100.0, 100.0, 10.0, 95.0)];
        let layout = SolverPageLayout::new(placements.clone(), canvas);

        let book_config = BookConfig {
            margin_mm: 0.0,
            bleed_mm: 5.0,
            bleed_threshold_mm: 5.0,
            ..Default::default()
        };

        let scale_factor = layout.calc_needed_scaling_around_center_for_bleed(&book_config);
        assert_relative_eq!(
            canvas.center().1
                + (canvas.center().1 - placements[0].y) * scale_factor
                + placements[0].h * scale_factor,
            205.0,
            epsilon = 1e-6
        );
    }

    #[test]
    fn test_calc_needed_cascading_scaling_around_center_for_bleed() {
        let canvas = Canvas::new(100.0, 100.0, 0.0);
        let placements = vec![PhotoPlacement::new(0, 5.0, 4.0, 92.0, 94.0)];
        let layout = SolverPageLayout::new(placements.clone(), canvas);

        let book_config = BookConfig {
            margin_mm: 0.0,
            bleed_mm: 2.0,
            bleed_threshold_mm: 2.0,
            ..Default::default()
        };

        let scale_factor = layout.calc_needed_scaling_around_center_for_bleed(&book_config);
        let expected_scale_factor = 52.0 / 45.0;
        assert_relative_eq!(scale_factor, expected_scale_factor, epsilon = 1e-6);
    }

    #[test]
    fn test_zoom_to_respect_bleed_cascading_scaling() {
        let canvas = Canvas::new(100.0, 100.0, 0.0);
        let placements = vec![PhotoPlacement::new(0, 5.0, 4.0, 92.0, 94.0)];
        let layout = SolverPageLayout::new(placements.clone(), canvas);

        let book_config = BookConfig {
            margin_mm: 0.0,
            bleed_mm: 2.0,
            bleed_threshold_mm: 2.0,
            ..Default::default()
        };

        let zoomed_layout = layout.zoom_to_respect_bleed(&book_config);
        let exp_scale_factor = 52.0 / 45.0;
        let (center_x, center_y) = canvas.center();

        let p = &zoomed_layout.placements[0];

        let expected_x = center_x + (placements[0].x - center_x) * exp_scale_factor;
        let expected_y = center_y + (placements[0].y - center_y) * exp_scale_factor;
        let expected_w = placements[0].w * exp_scale_factor;
        let expected_h = placements[0].h * exp_scale_factor;

        assert_relative_eq!(p.x, expected_x, epsilon = 1e-6);
        assert_relative_eq!(p.y, expected_y, epsilon = 1e-6);
        assert_relative_eq!(p.w, expected_w, epsilon = 1e-6);
        assert_relative_eq!(p.h, expected_h, epsilon = 1e-6);
    }
}
