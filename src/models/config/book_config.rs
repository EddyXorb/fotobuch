use serde::{Deserialize, Serialize};

use super::{AppendixConfig, CanvasConfig, CoverConfig};
use crate::models::PageSide;

/// Book-specific configuration (page dimensions, bleed, margins, gaps)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookConfig {
    pub title: String,
    pub page_width_mm: f64,
    pub page_height_mm: f64,
    /// Bleed in mm (added around the content area, cut off in final PDF)
    pub bleed_mm: f64,
    /// Bleed in mm on the inside (spine) edge of inner pages. `None` (`null`) → `bleed_mm`.
    ///
    /// Print services with facing pages (e.g. CEWE) expect 0 mm here. The cover
    /// is not affected. Which edge is the inside edge follows [`PageSide`].
    /// Always serialized (as `null` when unset) so `config set`, `config show`
    /// and the GUI config window can find the key.
    #[serde(default)]
    pub bleed_inside_mm: Option<f64>,
    /// Side of the first inner page: `right` (default, inside covers unprinted,
    /// e.g. CEWE) or `left` (product also prints the inside of the front cover).
    /// The sides alternate from there; see [`PageSide`].
    #[serde(default)]
    pub first_page_side: PageSide,
    #[serde(default = "default_margin_mm")]
    pub margin_mm: f64,
    #[serde(default = "default_gap_mm")]
    pub gap_mm: f64,
    /// Min distance to page edge to consider a photo "touching" the edge and thus needing bleed. Only active if margin_mm = 0.
    #[serde(default = "default_bleed_threshold_mm")]
    pub bleed_threshold_mm: f64,
    /// DPI for final image generation (default: 300)
    #[serde(default = "default_dpi")]
    pub dpi: f64,
    /// Cover configuration. Always present, defaults to inactive (active: false).
    #[serde(default)]
    pub cover: CoverConfig,
    /// Photo index (appendix) configuration.
    #[serde(default)]
    pub appendix: AppendixConfig,
}

impl BookConfig {
    /// Effective inside (spine) bleed of inner pages in mm.
    pub fn effective_bleed_inside_mm(&self) -> f64 {
        self.bleed_inside_mm.unwrap_or(self.bleed_mm)
    }

    /// `(left, right)` bleed in mm of an inner page on the given side.
    pub fn bleed_left_right_mm(&self, side: PageSide) -> (f64, f64) {
        side.to_left_right(self.effective_bleed_inside_mm(), self.bleed_mm)
    }
}

impl CanvasConfig for BookConfig {
    fn page_width_mm(&self) -> f64 {
        self.page_width_mm
    }
    fn page_height_mm(&self) -> f64 {
        self.page_height_mm
    }
    fn bleed_mm(&self) -> f64 {
        self.bleed_mm
    }
    fn margin_mm(&self) -> f64 {
        self.margin_mm
    }
    fn gap_mm(&self) -> f64 {
        self.gap_mm
    }
    fn bleed_threshold_mm(&self) -> f64 {
        self.bleed_threshold_mm
    }
}

fn default_margin_mm() -> f64 {
    0.0
}

fn default_gap_mm() -> f64 {
    5.0
}

fn default_bleed_threshold_mm() -> f64 {
    3.0
}

fn default_dpi() -> f64 {
    300.0
}

impl Default for BookConfig {
    fn default() -> Self {
        Self {
            title: "Untitled".into(),
            page_width_mm: 210.0,
            page_height_mm: 297.0,
            bleed_mm: 3.0,
            bleed_inside_mm: None,
            first_page_side: PageSide::default(),
            margin_mm: default_margin_mm(),
            gap_mm: default_gap_mm(),
            bleed_threshold_mm: default_bleed_threshold_mm(),
            dpi: default_dpi(),
            cover: CoverConfig::default(),
            appendix: AppendixConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bleed_inside_defaults_to_bleed() {
        let cfg = BookConfig::default();
        assert_eq!(cfg.effective_bleed_inside_mm(), cfg.bleed_mm);
        assert_eq!(cfg.bleed_left_right_mm(PageSide::Right), (3.0, 3.0));
    }

    #[test]
    fn bleed_inside_is_on_spine_edge() {
        let cfg = BookConfig {
            bleed_inside_mm: Some(0.0),
            ..BookConfig::default()
        };
        assert_eq!(cfg.bleed_left_right_mm(PageSide::Right), (0.0, 3.0));
        assert_eq!(cfg.bleed_left_right_mm(PageSide::Left), (3.0, 0.0));
    }

    #[test]
    fn bleed_inside_unset_serializes_as_null() {
        let yaml = "title: t\npage_width_mm: 290\npage_height_mm: 290\nbleed_mm: 3\n";
        let cfg: BookConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.bleed_inside_mm, None);
        assert!(
            serde_yaml::to_string(&cfg)
                .unwrap()
                .contains("bleed_inside_mm: null")
        );

        let cfg: BookConfig = serde_yaml::from_str(&format!("{yaml}bleed_inside_mm: 0\n")).unwrap();
        assert_eq!(cfg.bleed_inside_mm, Some(0.0));
    }

    #[test]
    fn config_without_bleed_inside_key_keeps_symmetric_bleed() {
        // YAML written before bleed_inside_mm existed.
        let yaml = "title: t\npage_width_mm: 290\npage_height_mm: 290\nbleed_mm: 3\n";
        let cfg: BookConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.effective_bleed_inside_mm(), 3.0);
        assert_eq!(cfg.bleed_left_right_mm(PageSide::Right), (3.0, 3.0));
        assert_eq!(cfg.bleed_left_right_mm(PageSide::Left), (3.0, 3.0));

        let cfg: BookConfig =
            serde_yaml::from_str(&format!("{yaml}bleed_inside_mm: null\n")).unwrap();
        assert_eq!(cfg.bleed_inside_mm, None);
        assert_eq!(cfg.effective_bleed_inside_mm(), 3.0);
    }

    #[test]
    fn first_page_side_defaults_to_right() {
        let yaml = "title: t\npage_width_mm: 290\npage_height_mm: 290\nbleed_mm: 3\n";
        let cfg: BookConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.first_page_side, PageSide::Right);
        assert!(
            serde_yaml::to_string(&cfg)
                .unwrap()
                .contains("first_page_side: right")
        );
        let cfg: BookConfig =
            serde_yaml::from_str(&format!("{yaml}first_page_side: left\n")).unwrap();
        assert_eq!(cfg.first_page_side, PageSide::Left);
    }
}
