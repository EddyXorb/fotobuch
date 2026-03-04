use chrono::NaiveDateTime;
use std::path::PathBuf;

/// A single photo with metadata.
#[derive(Debug, Clone)]
pub struct Photo {
    pub path: PathBuf,
    /// Timestamp from EXIF data, or derived from the folder name as fallback.
    pub timestamp: Option<NaiveDateTime>,
    /// Pixel dimensions, if readable.
    pub dimensions: Option<(u32, u32)>,
}

impl Photo {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            timestamp: None,
            dimensions: None,
        }
    }

    /// Whether the photo is in landscape orientation.
    pub fn is_landscape(&self) -> bool {
        self.dimensions.map(|(w, h)| w >= h).unwrap_or(true)
    }

    pub fn aspect_ratio(&self) -> f64 {
        self.dimensions
            .map(|(w, h)| w as f64 / h as f64)
            .unwrap_or(1.5)
    }
}

/// A group of photos that belong together (e.g. from the same folder/day).
#[derive(Debug)]
pub struct PhotoGroup {
    pub label: String,
    pub timestamp: Option<NaiveDateTime>,
    pub photos: Vec<Photo>,
}

/// One page in the final photobook layout.
#[derive(Debug)]
pub struct Page {
    /// Photos placed on this page with their exact position and size in mm.
    pub placements: Vec<Placement>,
}

/// A single placed image on a page.
#[derive(Debug, Clone)]
pub struct Placement {
    pub photo: Photo,
    /// X offset from top-left corner in mm.
    pub x_mm: f64,
    /// Y offset from top-left corner in mm.
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}

/// Configuration for the photobook layout.
#[derive(Debug, Clone)]
pub struct BookConfig {
    /// Page width in mm (default: 297 for A4 landscape).
    pub page_width_mm: f64,
    /// Page height in mm (default: 210 for A4 landscape).
    pub page_height_mm: f64,
    /// Margin on all sides in mm.
    pub margin_mm: f64,
    /// Gap between photos in mm.
    pub gap_mm: f64,
    /// Maximum photos per page.
    pub max_photos_per_page: usize,
}

impl Default for BookConfig {
    fn default() -> Self {
        Self {
            page_width_mm: 297.0,
            page_height_mm: 210.0,
            margin_mm: 10.0,
            gap_mm: 3.0,
            max_photos_per_page: 4,
        }
    }
}
