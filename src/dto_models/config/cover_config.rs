use serde::{Deserialize, Serialize};

use crate::dto_models::config::book_config::CanvasConfig;

/// Layout mode for the cover page.
///
/// Controls whether the GA solver or the deterministic cover solver is used for
/// page 0, and which fixed slot geometry the cover solver generates.
///
/// **Default:** `Free` — existing behaviour, GA solver optimises freely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum CoverMode {
    /// GA solver optimises the cover like any other page (existing behaviour).
    #[default]
    Free,
    /// One photo on the front panel, aspect-ratio preserved and centred.
    Front,
    /// One photo filling the entire front panel (may crop).
    FrontFull,
    /// One photo on the back panel, aspect-ratio preserved and centred.
    Back,
    /// One photo filling the entire back panel (may crop).
    BackFull,
    /// One photo spanning the full spread (over spine), aspect-ratio preserved and centred.
    Spread,
    /// One photo filling the full spread (may crop).
    SpreadFull,
    /// Two photos: slot 0 → front, slot 1 → back, aspect-ratio preserved and centred.
    Split,
    /// Two photos: slot 0 → front, slot 1 → back, each half fully filled (may crop).
    SplitFull,
}

impl CoverMode {
    /// Number of photo slots this mode requires.
    /// Returns `None` for `Free` (any count is valid).
    pub fn required_slots(self) -> Option<usize> {
        match self {
            CoverMode::Free => None,
            CoverMode::Split | CoverMode::SplitFull => Some(2),
            _ => Some(1),
        }
    }

    /// `true` when the GA solver should be used instead of the cover solver.
    pub fn is_free(self) -> bool {
        self == CoverMode::Free
    }

    /// `true` when the mode intentionally fills slots without preserving aspect ratio (cropping modes).
    /// AR mismatch between slot and photo is expected and should not trigger re-solving.
    pub fn allows_ar_mismatch(self) -> bool {
        matches!(
            self,
            CoverMode::FrontFull
                | CoverMode::BackFull
                | CoverMode::SpreadFull
                | CoverMode::SplitFull
        )
    }
}

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

impl Default for SpineConfig {
    fn default() -> Self {
        SpineConfig::Auto {
            spine_mm_per_10_pages: 1.4,
        }
    }
}

/// Cover configuration. Present only if the project has a cover page.
/// Absence of this block means no cover exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverConfig {
    /// Whether the first layout entry is treated as the cover page.
    /// false = cover block present but disabled.
    #[serde(default)]
    pub active: bool,
    /// Slot layout mode. `Free` delegates to the GA solver; all other modes use
    /// the deterministic cover solver and bypass the GA entirely.
    #[serde(default)]
    pub mode: CoverMode,
    /// Minimum gap in mm between a photo edge and the spine (for front/back/split modes).
    /// Ignored for spread modes. Default: 5.0.
    #[serde(default = "default_spine_clearance_mm")]
    pub spine_clearance_mm: f64,
    /// Spine configuration: auto or fixed.
    #[serde(default, flatten)]
    pub spine: SpineConfig,
    /// Total cover width in mm (front + back, without spine).
    #[serde(default)]
    pub front_back_width_mm: f64,
    /// Cover height in mm.
    #[serde(default)]
    pub height_mm: f64,
    /// Text printed on the spine. Defaults to `book.title` if absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spine_text: Option<String>,
    #[serde(default)]
    pub bleed_mm: f64,
    #[serde(default)]
    pub margin_mm: f64,
    #[serde(default)]
    pub gap_mm: f64,
    #[serde(default)]
    pub bleed_threshold_mm: f64,
}

fn default_spine_clearance_mm() -> f64 {
    5.0
}

impl Default for CoverConfig {
    fn default() -> Self {
        CoverConfig {
            active: false,
            mode: CoverMode::Split,
            spine_clearance_mm: default_spine_clearance_mm(),
            spine: SpineConfig::default(),
            front_back_width_mm: 0.0,
            height_mm: 0.0,
            spine_text: None,
            bleed_mm: 3.0,
            margin_mm: 0.0,
            gap_mm: 5.0,
            bleed_threshold_mm: 3.0,
        }
    }
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
            SpineConfig::Auto {
                spine_mm_per_10_pages,
            } => (inner_page_count as f64 / 10.0) * spine_mm_per_10_pages,
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
            mode: CoverMode::Free,
            spine_clearance_mm: 5.0,
            spine: SpineConfig::Auto {
                spine_mm_per_10_pages,
            },
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
            mode: CoverMode::Free,
            spine_clearance_mm: 5.0,
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

    #[test]
    fn cover_mode_required_slots() {
        assert_eq!(CoverMode::Free.required_slots(), None);
        assert_eq!(CoverMode::Front.required_slots(), Some(1));
        assert_eq!(CoverMode::FrontFull.required_slots(), Some(1));
        assert_eq!(CoverMode::Back.required_slots(), Some(1));
        assert_eq!(CoverMode::BackFull.required_slots(), Some(1));
        assert_eq!(CoverMode::Spread.required_slots(), Some(1));
        assert_eq!(CoverMode::SpreadFull.required_slots(), Some(1));
        assert_eq!(CoverMode::Split.required_slots(), Some(2));
        assert_eq!(CoverMode::SplitFull.required_slots(), Some(2));
    }

    #[test]
    fn cover_mode_is_free() {
        assert!(CoverMode::Free.is_free());
        assert!(!CoverMode::Front.is_free());
        assert!(!CoverMode::Split.is_free());
    }

    #[test]
    fn cover_mode_allows_ar_mismatch() {
        assert!(!CoverMode::Free.allows_ar_mismatch());
        assert!(!CoverMode::Front.allows_ar_mismatch());
        assert!(CoverMode::FrontFull.allows_ar_mismatch());
        assert!(!CoverMode::Back.allows_ar_mismatch());
        assert!(CoverMode::BackFull.allows_ar_mismatch());
        assert!(!CoverMode::Spread.allows_ar_mismatch());
        assert!(CoverMode::SpreadFull.allows_ar_mismatch());
        assert!(!CoverMode::Split.allows_ar_mismatch());
        assert!(CoverMode::SplitFull.allows_ar_mismatch());
    }

    #[test]
    fn cover_mode_serde_roundtrip() {
        let yaml = serde_yaml::to_string(&CoverMode::FrontFull).unwrap();
        assert_eq!(yaml.trim(), "front-full");
        let back: CoverMode = serde_yaml::from_str("back-full").unwrap();
        assert_eq!(back, CoverMode::BackFull);
        let free: CoverMode = serde_yaml::from_str("free").unwrap();
        assert_eq!(free, CoverMode::Free);
    }

    #[test]
    fn cover_mode_default_is_split() {
        let c = CoverConfig::default();
        assert_eq!(c.mode, CoverMode::Split);
    }

    #[test]
    fn spine_clearance_default() {
        let c = CoverConfig::default();
        assert!((c.spine_clearance_mm - 5.0).abs() < 1e-9);
    }
}
