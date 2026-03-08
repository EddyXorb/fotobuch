use super::canvas::Canvas;
use super::photo::Photo;
use crate::dto_models::{LayoutPage, Slot};

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

    /// Returns the area of the photo in mm².
    pub fn area(&self) -> f64 {
        self.w * self.h
    }

    /// Returns the aspect ratio of the photo (width / height).
    pub fn aspect_ratio(&self) -> f64 {
        self.w / self.h
    }

    /// Returns the right edge x-coordinate.
    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    /// Returns the bottom edge y-coordinate.
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }
}

/// Complete layout of a single page containing all photo placements.
#[derive(Debug, Clone)]
pub struct SolverPageLayout {
    /// All photo placements on the canvas.
    pub placements: Vec<PhotoPlacement>,

    /// Canvas dimensions and parameters.
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
    /// placements.
    ///
    /// # Returns
    ///
    /// A new `SolverPageLayout` with centered placements.
    pub fn centered(&self) -> Self {
        if self.placements.is_empty() {
            return self.clone();
        }

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

    /// Converts internal solver page layout to DTO layout page.
    ///
    /// # Arguments
    ///
    /// * `page_num` - Page number (1-based)
    /// * `photos` - Array of photos to map photo_idx to photo.id
    ///
    /// # Returns
    ///
    /// A `LayoutPage` containing photo IDs and slot positions.
    pub fn to_layout_page(&self, page_num: usize, photos: &[Photo]) -> LayoutPage {
        let photo_ids: Vec<String> = self
            .placements
            .iter()
            .map(|p| photos[p.photo_idx as usize].id.clone())
            .collect();

        let slots: Vec<Slot> = self
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

        let dto_page = layout.to_layout_page(1, &photos);

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

        let dto_page = layout.to_layout_page(2, &photos);

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

        let dto_page = layout.to_layout_page(3, &photos);

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
    }}