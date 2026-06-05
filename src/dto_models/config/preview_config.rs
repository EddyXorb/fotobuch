use serde::{Deserialize, Serialize};

/// Preview-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewConfig {
    #[serde(default = "default_show_filenames")]
    pub show_filenames: bool,
    #[serde(default = "default_max_preview_px")]
    pub max_preview_px: u32,
    /// Show red bleed border and blue margin border overlays
    #[serde(default = "default_show_borders")]
    pub show_borders: bool,
    /// Show slot address and area weight on each photo
    #[serde(default = "default_show_slot_info")]
    pub show_slot_info: bool,
    #[serde(default = "default_show_preview_watermark")]
    pub show_preview_watermark: bool,
    /// Whether `build`/`rebuild` should (re)write the preview PDF.
    /// Disabling this speeds up page rendering in the GUI, which renders pages
    /// directly and does not depend on the preview PDF being on disk.
    #[serde(default = "default_write_pdf")]
    pub write_pdf: bool,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            show_filenames: default_show_filenames(),
            max_preview_px: default_max_preview_px(),
            show_borders: default_show_borders(),
            show_slot_info: default_show_slot_info(),
            show_preview_watermark: default_show_preview_watermark(),
            write_pdf: default_write_pdf(),
        }
    }
}

fn default_show_filenames() -> bool {
    false
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

fn default_show_preview_watermark() -> bool {
    true
}

fn default_write_pdf() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_pdf_defaults_to_true() {
        assert!(PreviewConfig::default().write_pdf);
    }

    #[test]
    fn write_pdf_defaults_to_true_when_absent_in_yaml() {
        let cfg: PreviewConfig = serde_yaml::from_str("show_filenames: true").unwrap();
        assert!(cfg.write_pdf);
    }

    #[test]
    fn write_pdf_roundtrips_false() {
        let cfg: PreviewConfig = serde_yaml::from_str("write_pdf: false").unwrap();
        assert!(!cfg.write_pdf);
    }
}
