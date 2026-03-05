//! Domain types for the photobook layout solver.
//!
//! This module contains the core data structures without any behavior:
//! - `Photo`: Photo metadata (aspect ratio, area weight, group, timestamp)
//! - `Canvas`: Canvas dimensions and spacing parameters
//! - `PhotoPlacement`: Position and size of a photo on the canvas
//! - `LayoutResult`: Complete layout with all photo placements
//! - `FitnessWeights`: Weights for the genetic algorithm fitness function

use chrono::{DateTime, Utc};

// ============================================================================
// Photo
// ============================================================================

/// A single photo with metadata for layout optimization.
#[derive(Debug, Clone)]
pub struct Photo {
    /// Aspect ratio: width / height.
    pub aspect_ratio: f64,
    
    /// Relative importance for size distribution (default: 1.0).
    /// Higher values → photo should get more area.
    pub area_weight: f64,
    
    /// Group identifier (e.g., folder name, event).
    pub group: String,
    
    /// Timestamp from EXIF or folder name.
    pub timestamp: Option<DateTime<Utc>>,
}

impl Photo {
    /// Creates a new photo with the given aspect ratio.
    pub fn new(aspect_ratio: f64, area_weight: f64, group: String) -> Self {
        assert!(aspect_ratio > 0.0, "Aspect ratio must be positive");
        assert!(area_weight > 0.0, "Area weight must be positive");
        
        Self {
            aspect_ratio,
            area_weight,
            group,
            timestamp: None,
        }
    }
    
    /// Returns whether the photo is in landscape orientation (width >= height).
    pub fn is_landscape(&self) -> bool {
        self.aspect_ratio >= 1.0
    }
    
    /// Returns whether the photo is in portrait orientation (height > width).
    pub fn is_portrait(&self) -> bool {
        self.aspect_ratio < 1.0
    }
}

// ============================================================================
// Canvas
// ============================================================================

/// Canvas dimensions and spacing parameters for the photobook layout.
#[derive(Debug, Clone, Copy)]
pub struct Canvas {
    /// Canvas width in mm.
    pub width: f64,
    
    /// Canvas height in mm.
    pub height: f64,
    
    /// Gap between photos in mm (β in the algorithm).
    pub beta: f64,
    
    /// Bleed margin extending beyond the paper edge in mm.
    pub bleed: f64,
}

impl Canvas {
    /// Creates a new canvas with the given dimensions.
    pub fn new(width: f64, height: f64, beta: f64, bleed: f64) -> Self {
        assert!(width > 0.0, "Canvas width must be positive");
        assert!(height > 0.0, "Canvas height must be positive");
        assert!(beta >= 0.0, "Beta must be non-negative");
        assert!(bleed >= 0.0, "Bleed must be non-negative");
        
        Self {
            width,
            height,
            beta,
            bleed,
        }
    }
    
    /// Returns the total area of the canvas in mm².
    pub fn area(&self) -> f64 {
        self.width * self.height
    }
    
    /// Returns the aspect ratio of the canvas (width / height).
    pub fn aspect_ratio(&self) -> f64 {
        self.width / self.height
    }
}

impl Default for Canvas {
    fn default() -> Self {
        // A4 landscape: 297mm × 210mm
        Self {
            width: 297.0,
            height: 210.0,
            beta: 2.0,
            bleed: 3.0,
        }
    }
}

// ============================================================================
// PhotoPlacement & LayoutResult
// ============================================================================

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

// ============================================================================
// FitnessWeights
// ============================================================================

/// Weights for the fitness function components.
#[derive(Debug, Clone, Copy)]
pub struct FitnessWeights {
    /// Weight for size distribution cost C1.
    pub w_size: f64,
    
    /// Weight for canvas coverage cost C2.
    pub w_coverage: f64,
    
    /// Weight for barycenter centering cost C_bary.
    pub w_barycenter: f64,
    
    /// Weight for reading order cost C_order.
    pub w_order: f64,
}

impl FitnessWeights {
    /// Creates new fitness weights.
    pub fn new(w_size: f64, w_coverage: f64, w_barycenter: f64, w_order: f64) -> Self {
        assert!(w_size >= 0.0, "w_size must be non-negative");
        assert!(w_coverage >= 0.0, "w_coverage must be non-negative");
        assert!(w_barycenter >= 0.0, "w_barycenter must be non-negative");
        assert!(w_order >= 0.0, "w_order must be non-negative");
        
        Self {
            w_size,
            w_coverage,
            w_barycenter,
            w_order,
        }
    }
}

impl Default for FitnessWeights {
    /// Returns the default weights as specified in the algorithm document.
    fn default() -> Self {
        Self {
            w_size: 1.0,
            w_coverage: 0.15,
            w_barycenter: 0.5,
            w_order: 0.3,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    
    // Photo tests
    #[test]
    fn test_new_photo() {
        let photo = Photo::new(1.5, 1.0, "test".to_string());
        assert_eq!(photo.aspect_ratio, 1.5);
        assert_eq!(photo.area_weight, 1.0);
        assert_eq!(photo.group, "test");
        assert!(photo.timestamp.is_none());
    }
    
    #[test]
    #[should_panic(expected = "Aspect ratio must be positive")]
    fn test_new_photo_negative_aspect_ratio() {
        Photo::new(-1.0, 1.0, "test".to_string());
    }
    
    #[test]
    #[should_panic(expected = "Area weight must be positive")]
    fn test_new_photo_negative_area_weight() {
        Photo::new(1.5, -1.0, "test".to_string());
    }
    
    #[test]
    fn test_is_landscape() {
        let landscape = Photo::new(1.5, 1.0, "test".to_string());
        assert!(landscape.is_landscape());
        assert!(!landscape.is_portrait());
        
        let square = Photo::new(1.0, 1.0, "test".to_string());
        assert!(square.is_landscape());
        assert!(!square.is_portrait());
    }
    
    #[test]
    fn test_is_portrait() {
        let portrait = Photo::new(0.75, 1.0, "test".to_string());
        assert!(portrait.is_portrait());
        assert!(!portrait.is_landscape());
    }
    
    // Canvas tests
    #[test]
    fn test_new_canvas() {
        let canvas = Canvas::new(297.0, 210.0, 2.0, 3.0);
        assert_eq!(canvas.width, 297.0);
        assert_eq!(canvas.height, 210.0);
        assert_eq!(canvas.beta, 2.0);
        assert_eq!(canvas.bleed, 3.0);
    }
    
    #[test]
    #[should_panic(expected = "Canvas width must be positive")]
    fn test_new_canvas_negative_width() {
        Canvas::new(-100.0, 210.0, 2.0, 3.0);
    }
    
    #[test]
    #[should_panic(expected = "Canvas height must be positive")]
    fn test_new_canvas_negative_height() {
        Canvas::new(297.0, -210.0, 2.0, 3.0);
    }
    
    #[test]
    #[should_panic(expected = "Beta must be non-negative")]
    fn test_new_canvas_negative_beta() {
        Canvas::new(297.0, 210.0, -1.0, 3.0);
    }
    
    #[test]
    fn test_canvas_area() {
        let canvas = Canvas::new(297.0, 210.0, 2.0, 3.0);
        assert_relative_eq!(canvas.area(), 62370.0, epsilon = 1e-6);
    }
    
    #[test]
    fn test_canvas_aspect_ratio() {
        let canvas = Canvas::new(297.0, 210.0, 2.0, 3.0);
        assert_relative_eq!(canvas.aspect_ratio(), 1.414285714, epsilon = 1e-6);
    }
    
    #[test]
    fn test_canvas_default() {
        let canvas = Canvas::default();
        assert_eq!(canvas.width, 297.0);
        assert_eq!(canvas.height, 210.0);
        assert_eq!(canvas.beta, 2.0);
        assert_eq!(canvas.bleed, 3.0);
    }
    
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
    
    // LayoutResult tests
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
    
    // FitnessWeights tests
    #[test]
    fn test_new_weights() {
        let w = FitnessWeights::new(1.0, 0.15, 0.5, 0.3);
        assert_eq!(w.w_size, 1.0);
        assert_eq!(w.w_coverage, 0.15);
        assert_eq!(w.w_barycenter, 0.5);
        assert_eq!(w.w_order, 0.3);
    }
    
    #[test]
    #[should_panic(expected = "w_size must be non-negative")]
    fn test_new_weights_negative_size() {
        FitnessWeights::new(-1.0, 0.15, 0.5, 0.3);
    }
    
    #[test]
    fn test_default_weights() {
        let w = FitnessWeights::default();
        assert_eq!(w.w_size, 1.0);
        assert_eq!(w.w_coverage, 0.15);
        assert_eq!(w.w_barycenter, 0.5);
        assert_eq!(w.w_order, 0.3);
    }
    
    #[test]
    fn test_zero_weights() {
        let w = FitnessWeights::new(0.0, 0.0, 0.0, 0.0);
        assert_eq!(w.w_size, 0.0);
        assert_eq!(w.w_coverage, 0.0);
        assert_eq!(w.w_barycenter, 0.0);
        assert_eq!(w.w_order, 0.0);
    }
}
