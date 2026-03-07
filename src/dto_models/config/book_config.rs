use serde::{Deserialize, Serialize};

/// Book-specific configuration (page dimensions, bleed, margins, gaps)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookConfig {
    pub title: String,
    pub page_width_mm: f64,
    pub page_height_mm: f64,
    pub bleed_mm: f64,
    #[serde(default = "default_margin_mm")]
    pub margin_mm: f64,
    #[serde(default = "default_gap_mm")]
    pub gap_mm: f64,
    #[serde(default = "default_bleed_threshold_mm")]
    pub bleed_threshold_mm: f64,
}

fn default_margin_mm() -> f64 {
    10.0
}

fn default_gap_mm() -> f64 {
    5.0
}

fn default_bleed_threshold_mm() -> f64 {
    3.0
}
