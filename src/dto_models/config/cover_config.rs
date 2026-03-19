use serde::{Deserialize, Serialize};

/// Cover configuration. Present only if the project has a cover page.
/// Absence of this block means no cover exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverConfig {
    /// Whether the first layout entry is treated as the cover page.
    /// false = cover block present but disabled.
    #[serde(default)]
    pub active: bool,
    /// Spine thickness per 10 pages (linear interpolation).
    /// For double-page spreads use the value per 10 spreads;
    /// for single pages halve accordingly.
    pub spine_mm_per_10_pages: f64,
    /// Total cover width in mm (front + back, without spine).
    /// Defaults to `2 × book.page_width_mm` if absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_width_mm: Option<f64>,
    /// Cover page height in mm. Defaults to `book.page_height_mm` if absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_height_mm: Option<f64>,
    /// Text printed on the spine. Defaults to `book.title` if absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spine_text: Option<String>,
}

impl CoverConfig {
    /// Total cover spread width: page_width_mm (front+back) + spine.
    /// `page_width_mm` defaults to `2 × book_page_width_mm` when absent.
    pub fn resolved_width_mm(&self, book_page_width_mm: f64, inner_page_count: usize) -> f64 {
        let front_back = self.page_width_mm.unwrap_or(2.0 * book_page_width_mm);
        front_back + self.spine_width_mm(inner_page_count)
    }

    /// Resolved cover height: explicit value or fallback to inner page height.
    pub fn resolved_height_mm(&self, book_page_height_mm: f64) -> f64 {
        self.page_height_mm.unwrap_or(book_page_height_mm)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(spine_mm_per_10_pages: f64) -> CoverConfig {
        CoverConfig {
            active: true,
            spine_mm_per_10_pages,
            page_width_mm: None,
            page_height_mm: None,
            spine_text: None,
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
    fn resolved_width_fallback() {
        // default front+back = 2*210 = 420, spine(0) = 0 → 420
        let c = cfg(1.0);
        assert!((c.resolved_width_mm(210.0, 0) - 420.0).abs() < 1e-9);
    }

    #[test]
    fn resolved_width_includes_spine() {
        // default front+back = 2*210 = 420, spine(10, 1.4) = 1.4 → 421.4
        let c = cfg(1.4);
        assert!((c.resolved_width_mm(210.0, 10) - 421.4).abs() < 1e-9);
    }

    #[test]
    fn resolved_width_explicit() {
        // explicit front+back = 400, spine(0) = 0 → 400
        let c = CoverConfig { active: true, spine_mm_per_10_pages: 1.0, page_width_mm: Some(400.0), page_height_mm: None, spine_text: None };
        assert!((c.resolved_width_mm(210.0, 0) - 400.0).abs() < 1e-9);
    }

    #[test]
    fn resolved_spine_text_fallback() {
        let c = cfg(1.0);
        assert_eq!(c.resolved_spine_text("Mein Buch"), "Mein Buch");
    }

    #[test]
    fn resolved_spine_text_explicit() {
        let c = CoverConfig { active: true, spine_mm_per_10_pages: 1.0, page_width_mm: None, page_height_mm: None, spine_text: Some("Override".into()) };
        assert_eq!(c.resolved_spine_text("Mein Buch"), "Override");
    }
}
