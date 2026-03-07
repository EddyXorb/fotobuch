use serde::{Deserialize, Serialize};

use super::Slot;

/// Single page in the layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPage {
    /// Page number (1-based, for user reference only)
    pub page: usize,
    /// Photo IDs on this page (sorted by ratio)
    pub photos: Vec<String>,
    /// Calculated slot positions (index-coupled to photos)
    pub slots: Vec<Slot>,
}
