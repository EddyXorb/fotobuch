use crate::dto_models::BookConfig;

/// Canvas dimensions and spacing parameters for the photobook layout.
#[derive(Debug, Clone, Copy)]
pub struct Canvas {
    /// Canvas width in mm.
    pub width: f64,

    /// Canvas height in mm.
    pub height: f64,

    /// Gap between photos in mm (β in the algorithm).
    pub beta: f64,
}

impl Canvas {
    /// Creates a new canvas with the given dimensions.
    pub fn new(width: f64, height: f64, beta: f64) -> Self {
        assert!(width > 0.0, "Canvas width must be positive");
        assert!(height > 0.0, "Canvas height must be positive");
        assert!(beta >= 0.0, "Beta must be non-negative");

        Self {
            width,
            height,
            beta,
        }
    }

    /// Returns the total area of the canvas in mm².
    pub fn area(&self) -> f64 {
        self.width * self.height
    }

    /// Returns the aspect ratio of the canvas (width / height).
    #[allow(dead_code)]
    pub fn aspect_ratio(&self) -> f64 {
        self.width / self.height
    }

    /// Creates a Canvas from BookConfig with proper bleed/margin logic.
    ///
    /// Canvas dimensions are calculated as:
    /// - If margin = 0: canvas = page + 2*bleed (photos can extend into bleed area)
    /// - If margin > 0: canvas = page - 2*margin (bleed is outside margin, not relevant for layout)
    ///
    /// # Arguments
    ///
    /// * `config` - BookConfig with page dimensions, margin, gap, and bleed
    ///
    /// # Returns
    ///
    /// A new Canvas with dimensions calculated from BookConfig
    pub fn from_book_config(config: &BookConfig) -> Self {
        let width = if config.margin_mm == 0.0 {
            config.page_width_mm + 2.0 * config.bleed_mm
        } else {
            config.page_width_mm - 2.0 * config.margin_mm
        };

        let height = if config.margin_mm == 0.0 {
            config.page_height_mm + 2.0 * config.bleed_mm
        } else {
            config.page_height_mm - 2.0 * config.margin_mm
        };

        Self::new(width, height, config.gap_mm)
    }
}

impl Default for Canvas {
    fn default() -> Self {
        // A4 landscape: 297mm × 210mm
        Self {
            width: 297.0,
            height: 210.0,
            beta: 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::*;
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_new_canvas() {
        let canvas = standard_a4_canvas();
        assert_eq!(canvas.width, A4_WIDTH_MM);
        assert_eq!(canvas.height, A4_HEIGHT_MM);
        assert_eq!(canvas.beta, DEFAULT_GAP_MM);
    }

    #[test]
    #[should_panic(expected = "Canvas width must be positive")]
    fn test_new_canvas_negative_width() {
        Canvas::new(-100.0, A4_HEIGHT_MM, DEFAULT_GAP_MM);
    }

    #[test]
    #[should_panic(expected = "Canvas height must be positive")]
    fn test_new_canvas_negative_height() {
        Canvas::new(A4_WIDTH_MM, -210.0, DEFAULT_GAP_MM);
    }

    #[test]
    #[should_panic(expected = "Beta must be non-negative")]
    fn test_new_canvas_negative_beta() {
        Canvas::new(A4_WIDTH_MM, A4_HEIGHT_MM, -1.0);
    }

    #[test]
    fn test_canvas_area() {
        let canvas = standard_a4_canvas();
        assert_relative_eq!(canvas.area(), 62370.0, epsilon = 1e-6);
    }

    #[test]
    fn test_canvas_aspect_ratio() {
        let canvas = standard_a4_canvas();
        assert_relative_eq!(canvas.aspect_ratio(), 1.414285714, epsilon = 1e-6);
    }

    #[test]
    fn test_canvas_default() {
        let canvas = Canvas::default();
        assert_eq!(canvas.width, A4_WIDTH_MM);
        assert_eq!(canvas.height, A4_HEIGHT_MM);
        assert_eq!(canvas.beta, DEFAULT_GAP_MM);
    }

    // Converter tests
    mod converter_tests {
        use super::*;

        fn create_book_config(
            width: f64,
            height: f64,
            margin: f64,
            bleed: f64,
            gap: f64,
        ) -> BookConfig {
            BookConfig {
                title: "Test".to_string(),
                page_width_mm: width,
                page_height_mm: height,
                bleed_mm: bleed,
                margin_mm: margin,
                gap_mm: gap,
                bleed_threshold_mm: 3.0,
            }
        }

        #[test]
        fn test_from_book_config_with_margin() {
            let config = create_book_config(297.0, 210.0, 10.0, 3.0, 5.0);
            let canvas = Canvas::from_book_config(&config);

            // Canvas = page - 2*margin
            assert_relative_eq!(canvas.width, 277.0, epsilon = 1e-6);
            assert_relative_eq!(canvas.height, 190.0, epsilon = 1e-6);
            assert_eq!(canvas.beta, 5.0);
        }

        #[test]
        fn test_from_book_config_without_margin() {
            let config = create_book_config(297.0, 210.0, 0.0, 3.0, 5.0);
            let canvas = Canvas::from_book_config(&config);

            // Canvas = page + 2*bleed (when margin = 0)
            assert_relative_eq!(canvas.width, 303.0, epsilon = 1e-6);
            assert_relative_eq!(canvas.height, 216.0, epsilon = 1e-6);
            assert_eq!(canvas.beta, 5.0);
        }

        #[test]
        fn test_from_book_config_zero_bleed() {
            let config = create_book_config(297.0, 210.0, 10.0, 0.0, 2.0);
            let canvas = Canvas::from_book_config(&config);

            // Bleed doesn't matter when margin > 0
            assert_relative_eq!(canvas.width, 277.0, epsilon = 1e-6);
            assert_relative_eq!(canvas.height, 190.0, epsilon = 1e-6);
            assert_eq!(canvas.beta, 2.0);
        }

        #[test]
        fn test_from_book_config_large_bleed_no_margin() {
            let config = create_book_config(100.0, 100.0, 0.0, 10.0, 1.0);
            let canvas = Canvas::from_book_config(&config);

            // Large bleed increases canvas significantly when margin = 0
            assert_relative_eq!(canvas.width, 120.0, epsilon = 1e-6);
            assert_relative_eq!(canvas.height, 120.0, epsilon = 1e-6);
        }

        #[test]
        fn test_from_book_config_preserves_gap() {
            let configs = vec![
                create_book_config(297.0, 210.0, 10.0, 3.0, 2.0),
                create_book_config(297.0, 210.0, 0.0, 3.0, 5.0),
                create_book_config(297.0, 210.0, 5.0, 0.0, 10.0),
            ];

            for config in configs {
                let canvas = Canvas::from_book_config(&config);
                assert_eq!(canvas.beta, config.gap_mm);
            }
        }
    }
}
