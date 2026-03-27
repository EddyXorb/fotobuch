use serde::{Deserialize, Serialize};

/// Preview-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewConfig {
    #[serde(default = "default_show_filenames")]
    pub show_filenames: bool,
    #[serde(default = "default_show_page_numbers")]
    pub show_page_numbers: bool,
    #[serde(default = "default_max_preview_px")]
    pub max_preview_px: u32,
    /// Show red bleed border and blue margin border overlays
    #[serde(default = "default_show_borders")]
    pub show_borders: bool,
    /// Show slot address and area weight on each photo
    #[serde(default = "default_show_slot_info")]
    pub show_slot_info: bool,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            show_filenames: default_show_filenames(),
            show_page_numbers: default_show_page_numbers(),
            max_preview_px: default_max_preview_px(),
            show_borders: default_show_borders(),
            show_slot_info: default_show_slot_info(),
        }
    }
}

fn default_show_filenames() -> bool {
    true
}

fn default_show_page_numbers() -> bool {
    true
}

fn default_max_preview_px() -> u32 {
    800
}

fn default_show_borders() -> bool {
    true
}

fn default_show_slot_info() -> bool {
    true
}
