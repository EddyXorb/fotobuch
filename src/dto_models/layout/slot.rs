use serde::{Deserialize, Serialize, Serializer};

fn serialize_f64_round<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let rounded = (value * 100.0).round() / 100.0;
    serializer.serialize_f64(rounded)
}

/// Placement slot for a photo
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slot {
    /// X position in mm
    #[serde(serialize_with = "serialize_f64_round")]
    pub x_mm: f64,
    /// Y position in mm
    #[serde(serialize_with = "serialize_f64_round")]
    pub y_mm: f64,
    /// Width in mm
    #[serde(serialize_with = "serialize_f64_round")]
    pub width_mm: f64,
    /// Height in mm
    #[serde(serialize_with = "serialize_f64_round")]
    pub height_mm: f64,
}
