use super::canvas::Canvas;
use super::photo::Photo;
use crate::dto_models::{BookConfig, LayoutPage, Slot};

/// Placement of a single photo on the canvas.
#[derive(Debug, Clone, Copy)]
pub struct PhotoPlacement {
    /// Index of the photo in the input array.
    pub photo_idx: u16,

    /// X offset from top-left corner in mm.
    pub x: f64,

    /// Y offset from top-left corner in mm.
    pub y: f64,

    /// Width of the photo in mm.
    pub w: f64,

    /// Height of the photo in mm.
    pub h: f64,
}

impl PhotoPlacement {
    /// Creates a new photo placement.
    pub fn new(photo_idx: u16, x: f64, y: f64, w: f64, h: f64) -> Self {
        assert!(w > 0.0, "Width must be positive: {}", w);
        assert!(h > 0.0, "Height must be positive: {}", h);

        Self {
            photo_idx,
            x,
            y,
            w,
            h,
        }
    }

    /// Returns the center point of the photo.
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
    /// Moves the photo by the given offsets.
    #[allow(dead_code)]
    pub fn shift(&self, dx: f64, dy: f64) -> Self {
        Self {
            photo_idx: self.photo_idx,
            x: self.x + dx,
            y: self.y + dy,
            w: self.w,
            h: self.h,
        }
    }

    /// Scales the photo by fixing the top-left corner and scaling width and height.
    #[allow(dead_code)]
    pub fn scale(&self, factor: f64) -> Self {
        Self {
            photo_idx: self.photo_idx,
            x: self.x,
            y: self.y,
            w: self.w * factor,
            h: self.h * factor,
        }
    }

    /// Returns the area of the photo in mm².
    pub fn area(&self) -> f64 {
        self.w * self.h
    }

    /// Returns the aspect ratio of the photo (width / height).
    #[allow(dead_code)]
    pub fn aspect_ratio(&self) -> f64 {
        self.w / self.h
    }

    /// Returns the right edge x-coordinate.
    #[allow(dead_code)]
    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    /// Returns the bottom edge y-coordinate.
    #[allow(dead_code)]
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }
}

/// Complete layout of a single page containing all photo placements.
#[derive(Debug, Clone)]
pub struct SolverPageLayout {
    /// All photo placements on the canvas.
    pub placements: Vec<PhotoPlacement>,

    /// Canvas dimensions and parameters without bleed and margin.
    pub canvas: Canvas,
}

impl SolverPageLayout {
    /// Creates a new layout result.
    pub fn new(placements: Vec<PhotoPlacement>, canvas: Canvas) -> Self {
        Self { placements, canvas }
    }

    /// Returns the total area covered by all photos in mm².
    pub fn total_photo_area(&self) -> f64 {
        self.placements.iter().map(|p| p.area()).sum()
    }

    /// Returns the coverage ratio (0.0 to 1.0).
    pub fn coverage_ratio(&self) -> f64 {
        self.total_photo_area() / self.canvas.area()
    }

    /// Returns the barycenter (area-weighted center) of all photos.
    pub fn barycenter(&self) -> (f64, f64) {
        let total_area: f64 = self.placements.iter().map(|p| p.area()).sum();

        if total_area == 0.0 {
            return (self.canvas.width / 2.0, self.canvas.height / 2.0);
        }

        let weighted_x: f64 = self
            .placements
            .iter()
            .map(|p| {
                let (cx, _) = p.center();
                cx * p.area()
            })
            .sum();

        let weighted_y: f64 = self
            .placements
            .iter()
            .map(|p| {
                let (_, cy) = p.center();
                cy * p.area()
            })
            .sum();

        (weighted_x / total_area, weighted_y / total_area)
    }

    /// Centers the layout on its canvas by calculating offsets.
    ///
    /// The solver produces layouts that start at (0, 0). This method calculates
    /// the bounding box of all placements and returns a new layout with centered
    /// placements. Since the underlying solver maximizes coverage (resulting in a bounding box
    /// that is maximal with respect to the canvas), we do not have to
    /// worry about unused space on the sides.
    ///
    /// # Returns
    ///
    /// A new `SolverPageLayout` with centered placements.
    pub fn centered(&self) -> Self {
        if self.placements.is_empty() {
            return self.clone();
        }

        let [min_x, min_y, max_x, max_y] = self.bounding_box();

        let layout_width = max_x - min_x;
        let layout_height = max_y - min_y;

        // Calculate centering offsets
        let offset_x = (self.canvas.width - layout_width) / 2.0 - min_x;
        let offset_y = (self.canvas.height - layout_height) / 2.0 - min_y;

        // Apply offsets to all placements
        let centered_placements: Vec<PhotoPlacement> = self
            .placements
            .iter()
            .map(|p| PhotoPlacement::new(p.photo_idx, p.x + offset_x, p.y + offset_y, p.w, p.h))
            .collect();

        SolverPageLayout::new(centered_placements, self.canvas)
    }

    fn bounding_box(&self) -> [f64; 4] {
        // Calculate bounding box of the layout
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for p in &self.placements {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x + p.w);
            max_y = max_y.max(p.y + p.h);
        }
        [min_x, min_y, max_x, max_y]
    }

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

    /// Converts internal solver page layout to DTO layout page,
    /// including bleed/margin adjustments based on BookConfig and centering.
    ///
    /// # Arguments
    ///
    /// * `page_num` - Page number (1-based)
    /// * `photos` - Array of photos to map photo_idx to photo.id
    ///
    /// # Returns
    ///
    /// A `LayoutPage` containing photo IDs and slot positions.
    pub fn to_layout_page(
        &self,
        page_num: usize,
        photos: &[Photo],
        book_config: &BookConfig,
    ) -> LayoutPage {
        let adapted_layout = self.centered().zoom_to_respect_bleed(book_config);

        let photo_ids: Vec<String> = adapted_layout
            .placements
            .iter()
            .map(|p| photos[p.photo_idx as usize].id.clone())
            .collect();

        let slots: Vec<Slot> = adapted_layout
            .placements
            .iter()
            .map(|p| Slot {
                x_mm: p.x,
                y_mm: p.y,
                width_mm: p.w,
                height_mm: p.h,
            })
            .collect();

        LayoutPage {
            page: page_num,
            photos: photo_ids,
            slots,
        }
    }
    /// Calculates the needed scaling factor to add bleed around the page
    /// center if the layout is too close to the print border.
    /// The scaling is meant to be applied to the center of the layout,
    ///  so that the layout "zooms in" and touches the bleed-margins, if necessary.
    fn calc_needed_scaling_around_center_for_bleed(&self, book_config: &BookConfig) -> f64 {
        if book_config.margin_mm > 0.0 || book_config.bleed_mm == 0.0 {
            return 1.0;
        }
        let mut bleed_scale_factor = 1.0;
        let mut scale_factor_increase_last_iteration = 1.0;
        let mut bb = self.bounding_box();
        let (center_width, center_height) = self.canvas.center();

        loop {
            // we have to loop here, since increasing one dimension could lead to the other dimension being too close to the print border
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

            // this is the needed increase in either x or y dim to reach the outer bleed border (which is around the canvas)
            let needed_increase = border_distances
                .iter()
                .enumerate()
                .filter(|&(_, d)| {
                    d <= &book_config.bleed_threshold_mm && d >= &-book_config.bleed_mm
                })
                .map(|(i, d)| (i, f64::abs(-book_config.bleed_mm - d)))
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            if needed_increase.is_none() || needed_increase.unwrap().1 <= 0.001 {
                break;
            }
            let idx_with_max = needed_increase.unwrap().0;

            if idx_with_max % 2 == 0 {
                // Left or right border
                let distance_to_center = f64::abs(center_width - bb[idx_with_max]);
                scale_factor_increase_last_iteration =
                    (distance_to_center + needed_increase.unwrap().1) / distance_to_center;
                bleed_scale_factor *= scale_factor_increase_last_iteration;
            } else {
                // Top or bottom border
                let distance_to_center = f64::abs(center_height - bb[idx_with_max]);
                scale_factor_increase_last_iteration =
                    (distance_to_center + needed_increase.unwrap().1) / distance_to_center;
                bleed_scale_factor *= scale_factor_increase_last_iteration;
            }
        }

        bleed_scale_factor
    }

    /// This method zooms the layout in around the center (it possibly crops, too), to respect the bleed requirements.
    fn zoom_to_respect_bleed(&self, book_config: &BookConfig) -> Self {
        let scale_factor = self.calc_needed_scaling_around_center_for_bleed(book_config);
        let (center_x, center_y) = self.canvas.center();
        self.scale_around_fixpoint(scale_factor, center_x, center_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // PhotoPlacement tests
    #[test]
    fn test_photo_placement_new() {
        let p = PhotoPlacement::new(0, 10.0, 20.0, 100.0, 50.0);
        assert_eq!(p.photo_idx, 0);
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
        assert_eq!(p.w, 100.0);
        assert_eq!(p.h, 50.0);
    }

    #[test]
    #[should_panic(expected = "Width must be positive")]
    fn test_photo_placement_negative_width() {
        PhotoPlacement::new(0, 10.0, 20.0, -100.0, 50.0);
    }

    #[test]
    fn test_photo_placement_center() {
        let p = PhotoPlacement::new(0, 10.0, 20.0, 100.0, 50.0);
        let (cx, cy) = p.center();
        assert_relative_eq!(cx, 60.0, epsilon = 1e-6);
        assert_relative_eq!(cy, 45.0, epsilon = 1e-6);
    }

    #[test]
    fn test_photo_placement_area() {
        let p = PhotoPlacement::new(0, 10.0, 20.0, 100.0, 50.0);
        assert_relative_eq!(p.area(), 5000.0, epsilon = 1e-6);
    }

    #[test]
    fn test_photo_placement_aspect_ratio() {
        let p = PhotoPlacement::new(0, 10.0, 20.0, 100.0, 50.0);
        assert_relative_eq!(p.aspect_ratio(), 2.0, epsilon = 1e-6);
    }

    #[test]
    fn test_photo_placement_edges() {
        let p = PhotoPlacement::new(0, 10.0, 20.0, 100.0, 50.0);
        assert_relative_eq!(p.right(), 110.0, epsilon = 1e-6);
        assert_relative_eq!(p.bottom(), 70.0, epsilon = 1e-6);
    }

    // SolverPageLayout tests
    #[test]
    fn test_layout_result_coverage() {
        let canvas = Canvas::new(200.0, 100.0, 2.0);
        let placements = vec![
            PhotoPlacement::new(0, 0.0, 0.0, 100.0, 100.0), // 10000 mm²
            PhotoPlacement::new(1, 102.0, 0.0, 98.0, 100.0), // 9800 mm²
        ];
        let layout = SolverPageLayout::new(placements, canvas);

        assert_relative_eq!(layout.total_photo_area(), 19800.0, epsilon = 1e-6);
        assert_relative_eq!(layout.coverage_ratio(), 0.99, epsilon = 1e-6);
    }

    #[test]
    fn test_layout_result_barycenter() {
        let canvas = Canvas::new(200.0, 200.0, 0.0);
        let placements = vec![
            PhotoPlacement::new(0, 0.0, 0.0, 100.0, 100.0), // center: (50, 50), area: 10000
            PhotoPlacement::new(1, 100.0, 100.0, 100.0, 100.0), // center: (150, 150), area: 10000
        ];
        let layout = SolverPageLayout::new(placements, canvas);

        let (bx, by) = layout.barycenter();
        assert_relative_eq!(bx, 100.0, epsilon = 1e-6);
        assert_relative_eq!(by, 100.0, epsilon = 1e-6);
    }

    #[test]
    fn test_layout_result_barycenter_empty() {
        let canvas = Canvas::new(200.0, 100.0, 0.0);
        let layout = SolverPageLayout::new(vec![], canvas);

        let (bx, by) = layout.barycenter();
        assert_relative_eq!(bx, 100.0, epsilon = 1e-6);
        assert_relative_eq!(by, 50.0, epsilon = 1e-6);
    }

    // SolverPageLayout::centered() tests
    #[test]
    fn test_centered_offset() {
        let canvas = Canvas::new(500.0, 500.0, 2.0);
        let placements = vec![PhotoPlacement::new(0, 0.0, 0.0, 100.0, 100.0)];
        let layout = SolverPageLayout::new(placements, canvas);

        let centered = layout.centered();

        let p = &centered.placements[0];
        assert_relative_eq!(p.x, 200.0, epsilon = 1e-6);
        assert_relative_eq!(p.y, 200.0, epsilon = 1e-6);
    }

    #[test]
    fn test_centered_multiple_placements() {
        let canvas = Canvas::new(300.0, 300.0, 0.0);
        let placements = vec![
            PhotoPlacement::new(0, 0.0, 0.0, 100.0, 100.0),
            PhotoPlacement::new(1, 100.0, 0.0, 100.0, 100.0),
        ];
        let layout = SolverPageLayout::new(placements, canvas);

        let centered = layout.centered();

        assert_relative_eq!(centered.placements[0].x, 50.0, epsilon = 1e-6);
        assert_relative_eq!(centered.placements[0].y, 100.0, epsilon = 1e-6);
        assert_relative_eq!(centered.placements[1].x, 150.0, epsilon = 1e-6);
        assert_relative_eq!(centered.placements[1].y, 100.0, epsilon = 1e-6);
    }

    #[test]
    fn test_centered_empty() {
        let canvas = Canvas::new(200.0, 200.0, 2.0);
        let layout = SolverPageLayout::new(vec![], canvas);

        let centered = layout.centered();

        assert!(centered.placements.is_empty());
    }

    // SolverPageLayout::to_layout_page() tests
    #[test]
    fn test_to_layout_page_empty() {
        let canvas = Canvas::new(200.0, 200.0, 2.0);
        let layout = SolverPageLayout::new(vec![], canvas);
        let photos = vec![];

        let dto_page = layout.to_layout_page(1, &photos, &BookConfig::default());

        assert_eq!(dto_page.page, 1);
        assert!(dto_page.photos.is_empty());
        assert!(dto_page.slots.is_empty());
    }

    #[test]
    fn test_to_layout_page_single_photo() {
        let canvas = Canvas::new(200.0, 200.0, 0.0);
        let placements = vec![PhotoPlacement::new(0, 10.0, 20.0, 100.0, 80.0)];
        let layout = SolverPageLayout::new(placements, canvas);

        let photos = vec![Photo::new(
            "photo_abc".to_string(),
            1.5,
            1.0,
            "group1".to_string(),
        )];

        let dto_page = layout.to_layout_page(2, &photos, &BookConfig::default());

        assert_eq!(dto_page.page, 2);
        assert_eq!(dto_page.photos.len(), 1);
        assert_eq!(dto_page.photos[0], "photo_abc");
        assert_eq!(dto_page.slots.len(), 1);
        assert_relative_eq!(dto_page.slots[0].x_mm, 10.0, epsilon = 1e-6);
        assert_relative_eq!(dto_page.slots[0].y_mm, 20.0, epsilon = 1e-6);
        assert_relative_eq!(dto_page.slots[0].width_mm, 100.0, epsilon = 1e-6);
        assert_relative_eq!(dto_page.slots[0].height_mm, 80.0, epsilon = 1e-6);
    }

    #[test]
    fn test_to_layout_page_multiple_photos() {
        let canvas = Canvas::new(300.0, 300.0, 0.0);
        let placements = vec![
            PhotoPlacement::new(0, 0.0, 0.0, 150.0, 100.0),
            PhotoPlacement::new(1, 150.0, 0.0, 150.0, 100.0),
            PhotoPlacement::new(2, 0.0, 100.0, 300.0, 200.0),
        ];
        let layout = SolverPageLayout::new(placements, canvas);

        let photos = vec![
            Photo::new("id_1".to_string(), 1.5, 1.0, "group1".to_string()),
            Photo::new("id_2".to_string(), 1.5, 1.0, "group1".to_string()),
            Photo::new("id_3".to_string(), 1.5, 1.0, "group2".to_string()),
        ];

        let dto_page = layout.to_layout_page(3, &photos, &BookConfig::default());

        assert_eq!(dto_page.page, 3);
        assert_eq!(dto_page.photos, vec!["id_1", "id_2", "id_3"]);
        assert_eq!(dto_page.slots.len(), 3);

        // Check first slot
        assert_relative_eq!(dto_page.slots[0].x_mm, 0.0, epsilon = 1e-6);
        assert_relative_eq!(dto_page.slots[0].y_mm, 0.0, epsilon = 1e-6);
        assert_relative_eq!(dto_page.slots[0].width_mm, 150.0, epsilon = 1e-6);
        assert_relative_eq!(dto_page.slots[0].height_mm, 100.0, epsilon = 1e-6);

        // Check second slot
        assert_relative_eq!(dto_page.slots[1].x_mm, 150.0, epsilon = 1e-6);
        assert_relative_eq!(dto_page.slots[1].y_mm, 0.0, epsilon = 1e-6);

        // Check third slot
        assert_relative_eq!(dto_page.slots[2].x_mm, 0.0, epsilon = 1e-6);
        assert_relative_eq!(dto_page.slots[2].y_mm, 100.0, epsilon = 1e-6);
        assert_relative_eq!(dto_page.slots[2].width_mm, 300.0, epsilon = 1e-6);
        assert_relative_eq!(dto_page.slots[2].height_mm, 200.0, epsilon = 1e-6);
    }

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
        assert_relative_eq!(scale_factor, 105.0 / 95.0 , epsilon = 1e-6);
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

        // Verify the scaling was applied correctly
        let p = &zoomed_layout.placements[0];

        // Calculate expected positions after scaling around the center
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
