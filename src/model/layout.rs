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
        assert!(w > 0.0, "Width must be positive");
        assert!(h > 0.0, "Height must be positive");
        
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

/// Complete layout result containing all photo placements.
#[derive(Debug, Clone)]
pub struct LayoutResult {
    /// All photo placements on the canvas.
    pub placements: Vec<PhotoPlacement>,
    
    /// Canvas dimensions and parameters.
    pub canvas: Canvas,
}

impl LayoutResult {
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
        
        let weighted_x: f64 = self.placements
            .iter()
            .map(|p| {
                let (cx, _) = p.center();
                cx * p.area()
            })
            .sum();
        
        let weighted_y: f64 = self.placements
            .iter()
            .map(|p| {
                let (_, cy) = p.center();
                cy * p.area()
            })
            .sum();
        
        (weighted_x / total_area, weighted_y / total_area)
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
        let canvas = Canvas::new(200.0, 100.0, 2.0, 3.0);
        let placements = vec![
            PhotoPlacement::new(0, 0.0, 0.0, 100.0, 100.0),  // 10000 mm²
            PhotoPlacement::new(1, 102.0, 0.0, 98.0, 100.0), // 9800 mm²
        ];
        let layout = LayoutResult::new(placements, canvas);
        
        assert_relative_eq!(layout.total_photo_area(), 19800.0, epsilon = 1e-6);
        assert_relative_eq!(layout.coverage_ratio(), 0.99, epsilon = 1e-6);
    }
    
    #[test]
    fn test_layout_result_barycenter() {
        let canvas = Canvas::new(200.0, 200.0, 0.0, 0.0);
        let placements = vec![
            PhotoPlacement::new(0, 0.0, 0.0, 100.0, 100.0),    // center: (50, 50), area: 10000
            PhotoPlacement::new(1, 100.0, 100.0, 100.0, 100.0), // center: (150, 150), area: 10000
        ];
        let layout = LayoutResult::new(placements, canvas);
        
        let (bx, by) = layout.barycenter();
        assert_relative_eq!(bx, 100.0, epsilon = 1e-6);
        assert_relative_eq!(by, 100.0, epsilon = 1e-6);
    }
    
    #[test]
    fn test_layout_result_barycenter_weighted() {
        let canvas = Canvas::new(300.0, 100.0, 0.0, 0.0);
        let placements = vec![
            PhotoPlacement::new(0, 0.0, 0.0, 100.0, 100.0),    // center: (50, 50), area: 10000
            PhotoPlacement::new(1, 100.0, 0.0, 200.0, 100.0),  // center: (200, 50), area: 20000
        ];
        let layout = LayoutResult::new(placements, canvas);
        
        let (bx, by) = layout.barycenter();
        // bx = (50 * 10000 + 200 * 20000) / 30000 = 4500000 / 30000 = 150
        assert_relative_eq!(bx, 150.0, epsilon = 1e-6);
        assert_relative_eq!(by, 50.0, epsilon = 1e-6);
    }
    
    #[test]
    fn test_layout_result_barycenter_empty() {
        let canvas = Canvas::new(200.0, 100.0, 0.0, 0.0);
        let layout = LayoutResult::new(vec![], canvas);
        
        let (bx, by) = layout.barycenter();
        assert_relative_eq!(bx, 100.0, epsilon = 1e-6);
        assert_relative_eq!(by, 50.0, epsilon = 1e-6);
    }
}
