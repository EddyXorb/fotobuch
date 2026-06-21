use serde::{Deserialize, Serialize};

use super::Slot;

/// Page mode: Auto (solver places photos) or Manual (user places photos manually)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PageMode {
    #[default]
    Auto,
    Manual,
}

fn is_auto(mode: &PageMode) -> bool {
    *mode == PageMode::Auto
}

/// Single page in the layout.
/// Margin and bleed are **considered** in the slot positions,
/// so they are absolute coordinates respecting those.
/// The photos are placed within the box (the Trimbox in the PDF sense):
/// (bleed+margin,bleed+margin,page_width-bleed-margin,page_height-bleed-margin).
///
/// The page's index in `layout[]` is its canonical identity — no redundant
/// `page` field is stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPage {
    /// Photo IDs on this page (sorted by ratio)
    pub photos: Vec<String>,
    /// Calculated slot positions (index-coupled to photos)
    pub slots: Vec<Slot>,
    /// Page mode: Auto or Manual. Missing in YAML → Auto (backward compatibility
    /// via `#[serde(default)]` + `#[default]` on `PageMode`). Auto is skipped
    /// when serializing, so existing YAMLs stay clean.
    #[serde(default, skip_serializing_if = "is_auto")]
    pub mode: PageMode,
}
