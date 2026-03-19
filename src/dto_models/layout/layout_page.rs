use serde::{Deserialize, Serialize};

use super::Slot;

/// Single page in the layout.
/// Margin and bleed are **considered** in the slot positions,
/// so they are absolute coordinates respecting those.
/// The photos are placed within the box (the Trimbox in the PDF sense):
/// (bleed+margin,bleed+margin,page_width-bleed-margin,page_height-bleed-margin).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPage {
    /// Page number. Without cover: 1-based (= index + 1).
    /// With cover: cover page = 0, inner pages 1-based (= index).
    pub page: usize,
    /// Photo IDs on this page (sorted by ratio)
    pub photos: Vec<String>,
    /// Calculated slot positions (index-coupled to photos)
    pub slots: Vec<Slot>,
}
