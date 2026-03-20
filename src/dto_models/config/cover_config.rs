use serde::{Deserialize, Serialize};

use crate::dto_models::config::book_config::CanvasConfig;

/// Cover configuration. Present only if the project has a cover page.
/// Absence of this block means no cover exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverConfig {
    /// Whether the first layout entry is treated as the cover page.
    /// false = cover block present but disabled.
    #[serde(default)]
    pub active: bool,
    /// Spine thickness per 10 pages (linear interpolation).
    pub spine_mm_per_10_pages: f64,
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
    /// Total cover spread width: front_back_width_mm + spine.
    pub fn spread_width_mm(&self, inner_page_count: usize) -> f64 {
        self.front_back_width_mm + self.spine_width_mm(inner_page_count)
    }

    /// Resolved spine text: explicit value or fallback to book title.
    pub fn resolved_spine_text<'a>(&'a self, book_title: &'a str) -> &'a str {
        self.spine_text.as_deref().unwrap_or(book_title)
    }

    /// Calculated spine width in mm given the number of inner pages.
    pub fn spine_width_mm(&self, inner_page_count: usize) -> f64 {
        (inner_page_count as f64 / 10.0) * self.spine_mm_per_10_pages
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

    fn cfg(spine_mm_per_10_pages: f64) -> CoverConfig {
        CoverConfig {
            active: true,
            spine_mm_per_10_pages,
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
    fn spine_width_linear() {
        let c = cfg(1.4);
        assert!((c.spine_width_mm(10) - 1.4).abs() < 1e-9);
        assert!((c.spine_width_mm(100) - 14.0).abs() < 1e-9);
        assert!((c.spine_width_mm(0) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn spread_width_no_spine() {
        let c = cfg(1.0);
        assert!((c.spread_width_mm(0) - 420.0).abs() < 1e-9);
    }

    #[test]
    fn spread_width_includes_spine() {
        let c = cfg(1.4);
        assert!((c.spread_width_mm(10) - 421.4).abs() < 1e-9);
    }

    #[test]
    fn resolved_spine_text_fallback() {
        let c = cfg(1.0);
        assert_eq!(c.resolved_spine_text("Mein Buch"), "Mein Buch");
    }

    #[test]
    fn resolved_spine_text_explicit() {
        let mut c = cfg(1.0);
        c.spine_text = Some("Override".into());
        assert_eq!(c.resolved_spine_text("Mein Buch"), "Override");
    }
}
