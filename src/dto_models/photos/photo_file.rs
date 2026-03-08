use chrono::{DateTime, Utc};
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
    /// Timestamp for chronological ordering (ISO 8601)
    pub timestamp: DateTime<Utc>,
    /// Blake3 hash for duplicate detection (hex string, 64 chars)
    pub hash: String,
}

fn default_area_weight() -> f64 {
    1.0
}

impl PhotoFile {
    /// Returns the aspect ratio (width / height) of the photo.
    pub fn aspect_ratio(&self) -> f64 {
        self.width_px as f64 / self.height_px as f64
    }
}
