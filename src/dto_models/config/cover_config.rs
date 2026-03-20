use serde::{Deserialize, Serialize};

use crate::dto_models::config::book_config::CanvasConfig;

/// Spine configuration: auto-calculated from page count or fixed by user.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "spine_mode", rename_all = "snake_case")]
pub enum SpineConfig {
    /// Spine width calculated from inner page count: (pages / 10) * spine_mm_per_10_pages.
    /// Affects cover total width.
    Auto { spine_mm_per_10_pages: f64 },
    /// Fixed spine width provided by user. Does NOT affect cover total width in solver,
    /// but is used by the template for display and text sizing.
    Fixed { spine_width_mm: f64 },
}

/// Cover configuration. Present only if the project has a cover page.
/// Absence of this block means no cover exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverConfig {
    /// Whether the first layout entry is treated as the cover page.
    /// false = cover block present but disabled.
    #[serde(default)]
    pub active: bool,
    /// Spine configuration: auto or fixed.
    #[serde(flatten)]
    pub spine: SpineConfig,
    /// Total cover width in mm (front + back, without spine).
    pub front_back_width_mm: f64,
    /// Cover height in mm.
    pub height_mm: f64,
    /// Text printed on the spine. Defaults to `book.title` if absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spine_text: Option<String>,
    pub bleed_mm: f64,
    pub margin_mm: f64,
    pub gap_mm: f64,
    pub bleed_threshold_mm: f64,
}

impl CoverConfig {
    /// Total cover spread width in auto mode: front_back_width_mm + spine.
    /// In fixed mode: only front_back_width_mm (spine doesn't affect solver canvas).
    pub fn spread_width_mm(&self, inner_page_count: usize) -> f64 {
        match &self.spine {
            SpineConfig::Auto { .. } => {
                self.front_back_width_mm + self.spine_width_mm(inner_page_count)
            }
            SpineConfig::Fixed { .. } => {
                // Fixed spine does not affect cover width
                self.front_back_width_mm
            }
        }
    }

    /// Resolved spine text: explicit value or fallback to book title.
    pub fn resolved_spine_text<'a>(&'a self, book_title: &'a str) -> &'a str {
        self.spine_text.as_deref().unwrap_or(book_title)
    }

    /// Spine width in mm. In auto mode, calculated from page count. In fixed mode, the provided value.
    /// Used by template for display/text sizing in both modes.
    pub fn spine_width_mm(&self, inner_page_count: usize) -> f64 {
        match &self.spine {
            SpineConfig::Auto { spine_mm_per_10_pages } => {
                (inner_page_count as f64 / 10.0) * spine_mm_per_10_pages
            }
            SpineConfig::Fixed { spine_width_mm } => *spine_width_mm,
        }
    }
}

impl CanvasConfig for CoverConfig {
    /// Returns the full spread width (front + back + spine).
    /// Caller must pass the correct inner_page_count via spread_width_mm() when
    /// constructing the solver canvas — this value excludes the spine.
    fn page_width_mm(&self) -> f64 {
        self.front_back_width_mm
    }
    fn page_height_mm(&self) -> f64 {
        self.height_mm
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_auto(spine_mm_per_10_pages: f64) -> CoverConfig {
        CoverConfig {
            active: true,
            spine: SpineConfig::Auto { spine_mm_per_10_pages },
            front_back_width_mm: 420.0,
            height_mm: 297.0,
            spine_text: None,
            bleed_mm: 3.0,
            margin_mm: 0.0,
            gap_mm: 5.0,
            bleed_threshold_mm: 3.0,
        }
    }

    fn cfg_fixed(spine_width_mm: f64) -> CoverConfig {
        CoverConfig {
            active: true,
            spine: SpineConfig::Fixed { spine_width_mm },
            front_back_width_mm: 420.0,
            height_mm: 297.0,
            spine_text: None,
            bleed_mm: 3.0,
            margin_mm: 0.0,
            gap_mm: 5.0,
            bleed_threshold_mm: 3.0,
        }
    }

    #[test]
    fn spine_width_auto_linear() {
        let c = cfg_auto(1.4);
        assert!((c.spine_width_mm(10) - 1.4).abs() < 1e-9);
        assert!((c.spine_width_mm(100) - 14.0).abs() < 1e-9);
        assert!((c.spine_width_mm(0) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn spine_width_fixed() {
        let c = cfg_fixed(2.5);
        assert!((c.spine_width_mm(10) - 2.5).abs() < 1e-9);
        assert!((c.spine_width_mm(100) - 2.5).abs() < 1e-9);
        assert!((c.spine_width_mm(0) - 2.5).abs() < 1e-9);
    }

    #[test]
    fn spread_width_auto_no_spine() {
        let c = cfg_auto(1.0);
        assert!((c.spread_width_mm(0) - 420.0).abs() < 1e-9);
    }

    #[test]
    fn spread_width_auto_includes_spine() {
        let c = cfg_auto(1.4);
        assert!((c.spread_width_mm(10) - 421.4).abs() < 1e-9);
    }

    #[test]
    fn spread_width_fixed_ignores_spine() {
        let c = cfg_fixed(2.5);
        // Fixed mode: spread width is always front_back_width, regardless of page count
        assert!((c.spread_width_mm(10) - 420.0).abs() < 1e-9);
        assert!((c.spread_width_mm(100) - 420.0).abs() < 1e-9);
    }

    #[test]
    fn resolved_spine_text_fallback() {
        let c = cfg_auto(1.0);
        assert_eq!(c.resolved_spine_text("Mein Buch"), "Mein Buch");
    }

    #[test]
    fn resolved_spine_text_explicit() {
        let mut c = cfg_auto(1.0);
        c.spine_text = Some("Override".into());
        assert_eq!(c.resolved_spine_text("Mein Buch"), "Override");
    }
}
