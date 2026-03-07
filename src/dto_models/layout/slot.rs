use serde::{Deserialize, Serialize};

/// Placement slot for a photo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    /// X position in mm
    pub x_mm: f64,
    /// Y position in mm
    pub y_mm: f64,
    /// Width in mm
    pub width_mm: f64,
    /// Height in mm
    pub height_mm: f64,
}
