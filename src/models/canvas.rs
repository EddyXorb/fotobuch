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

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    
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
}
