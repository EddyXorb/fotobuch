use serde::{Deserialize, Serialize};

/// Individual photo with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoFile {
    /// Unique photo ID (used in layout)
    pub id: String,
    /// Absolute path to original file
    pub source: String,
    /// Width in pixels
    pub width_px: u32,
    /// Height in pixels
    pub height_px: u32,
    /// Area weight for solver (default: 1.0)
    #[serde(default = "default_area_weight")]
    pub area_weight: f64,
    /// Hash for duplicate detection (not serialized to YAML)
    #[serde(skip)]
    pub hash: Option<String>,
}

fn default_area_weight() -> f64 {
    1.0
}
