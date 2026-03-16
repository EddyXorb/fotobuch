use serde::{Deserialize, Serialize};

/// Book-specific configuration (page dimensions, bleed, margins, gaps)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookConfig {
    pub title: String,
    pub page_width_mm: f64,
    pub page_height_mm: f64,
    /// Bleed in mm (added around the content area, cut off in final PDF)
    pub bleed_mm: f64,
    #[serde(default = "default_margin_mm")]
    pub margin_mm: f64,
    #[serde(default = "default_gap_mm")]
    pub gap_mm: f64,
    /// Min distance to page edge to consider a photo "touching" the edge and thus needing bleed. Only active if margin_mm = 0.
    #[serde(default = "default_bleed_threshold_mm")]
    pub bleed_threshold_mm: f64,
    /// DPI for final image generation (default: 300)
    #[serde(default = "default_dpi")]
    pub dpi: f64,
}

fn default_margin_mm() -> f64 {
    0.0
}

fn default_gap_mm() -> f64 {
    5.0
}

fn default_bleed_threshold_mm() -> f64 {
    3.0
}

fn default_dpi() -> f64 {
    300.0
}

impl Default for BookConfig {
    fn default() -> Self {
        Self {
            title: "Untitled".into(),
            page_width_mm: 210.0,
            page_height_mm: 297.0,
            bleed_mm: 3.0,
            margin_mm: default_margin_mm(),
            gap_mm: default_gap_mm(),
            bleed_threshold_mm: default_bleed_threshold_mm(),
            dpi: default_dpi(),
        }
    }
}
