use super::canvas::Canvas;

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
        crate::models::aspect_ratio(self.w, self.h)
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
    pub fn centered(&self) -> Self {
        if self.placements.is_empty() {
            return self.clone();
        }

        let [min_x, min_y, max_x, max_y] = self.bounding_box();

        let layout_width = max_x - min_x;
        let layout_height = max_y - min_y;

        let offset_x = (self.canvas.width - layout_width) / 2.0 - min_x;
        let offset_y = (self.canvas.height - layout_height) / 2.0 - min_y;

        let centered_placements: Vec<PhotoPlacement> = self
            .placements
            .iter()
            .map(|p| PhotoPlacement::new(p.photo_idx, p.x + offset_x, p.y + offset_y, p.w, p.h))
            .collect();

        SolverPageLayout::new(centered_placements, self.canvas)
    }

    pub(crate) fn bounding_box(&self) -> [f64; 4] {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

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

    #[test]
    fn test_layout_result_coverage() {
        let canvas = Canvas::new(200.0, 100.0, 2.0);
        let placements = vec![
            PhotoPlacement::new(0, 0.0, 0.0, 100.0, 100.0),
            PhotoPlacement::new(1, 102.0, 0.0, 98.0, 100.0),
        ];
        let layout = SolverPageLayout::new(placements, canvas);

        assert_relative_eq!(layout.total_photo_area(), 19800.0, epsilon = 1e-6);
        assert_relative_eq!(layout.coverage_ratio(), 0.99, epsilon = 1e-6);
    }

    #[test]
    fn test_layout_result_barycenter() {
        let canvas = Canvas::new(200.0, 200.0, 0.0);
        let placements = vec![
            PhotoPlacement::new(0, 0.0, 0.0, 100.0, 100.0),
            PhotoPlacement::new(1, 100.0, 100.0, 100.0, 100.0),
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
}
