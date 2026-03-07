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
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            show_filenames: default_show_filenames(),
            show_page_numbers: default_show_page_numbers(),
            max_preview_px: default_max_preview_px(),
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
