//! Test fixtures and constants for unit tests.
//!
//! This module provides commonly used test data to reduce duplication
//! and improve test maintainability.

#![cfg(test)]
#![allow(dead_code)] // Not all fixtures are used in all test files yet

use crate::models::{Canvas, Photo};

// Canvas dimensions (A4 page in mm)
pub const A4_WIDTH_MM: f64 = 297.0;
pub const A4_HEIGHT_MM: f64 = 210.0;

// Common canvas parameters
pub const DEFAULT_GAP_MM: f64 = 2.0;
pub const DEFAULT_BLEED_MM: f64 = 3.0;
pub const TEST_GAP_MM: f64 = 5.0;
pub const TEST_BLEED_ZERO: f64 = 0.0;

// Photo aspect ratios
pub const LANDSCAPE_ASPECT: f64 = 1.5; // 3:2 ratio
pub const PORTRAIT_ASPECT: f64 = 0.75; // 2:3 ratio
pub const SQUARE_ASPECT: f64 = 1.0;

// Photo area weights
pub const DEFAULT_AREA_WEIGHT: f64 = 1.0;

/// Creates a standard A4 canvas for testing with typical gap and bleed values.
pub fn standard_a4_canvas() -> Canvas {
    Canvas::new(A4_WIDTH_MM, A4_HEIGHT_MM, DEFAULT_GAP_MM, DEFAULT_BLEED_MM)
}

/// Creates an A4 canvas with no bleed (useful for layout tests).
pub fn a4_canvas_with_gap(gap_mm: f64) -> Canvas {
    Canvas::new(A4_WIDTH_MM, A4_HEIGHT_MM, gap_mm, TEST_BLEED_ZERO)
}

/// Creates a standard landscape photo for testing.
pub fn landscape_photo(group: &str) -> Photo {
    Photo::new(LANDSCAPE_ASPECT, DEFAULT_AREA_WEIGHT, group.to_string())
}

/// Creates a standard portrait photo for testing.
pub fn portrait_photo(group: &str) -> Photo {
    Photo::new(PORTRAIT_ASPECT, DEFAULT_AREA_WEIGHT, group.to_string())
}

/// Creates a square photo for testing.
pub fn square_photo(group: &str) -> Photo {
    Photo::new(SQUARE_ASPECT, DEFAULT_AREA_WEIGHT, group.to_string())
}

/// Creates a photo with custom area weight.
pub fn weighted_photo(aspect: f64, weight: f64, group: &str) -> Photo {
    Photo::new(aspect, weight, group.to_string())
}
