use crate::models::config::{CanvasConfig, CoverConfig, SpineConfig};

/// Geometry value object: binds a `CoverConfig` to a concrete `inner_page_count`
/// and owns all dimension calculations.
///
/// `CoverConfig` is a pure data DTO; this type holds the calculations that require
/// the runtime value `inner_page_count`.
pub struct CoverGeometry<'a> {
    cover: &'a CoverConfig,
    inner_page_count: usize,
}

impl<'a> CoverGeometry<'a> {
    pub fn new(cover: &'a CoverConfig, inner_page_count: usize) -> Self {
        Self {
            cover,
            inner_page_count,
        }
    }

    /// Total spread width: front + back + spine (auto mode) or front + back only (fixed mode).
    pub fn spread_width_mm(&self) -> f64 {
        match &self.cover.spine {
            SpineConfig::Auto { .. } => self.cover.front_back_width_mm + self.spine_width_mm(),
            SpineConfig::Fixed { .. } => self.cover.front_back_width_mm,
        }
    }

    /// Spine width: calculated from page count (auto) or user-supplied (fixed).
    pub fn spine_width_mm(&self) -> f64 {
        match &self.cover.spine {
            SpineConfig::Auto {
                spine_mm_per_10_pages,
            } => (self.inner_page_count as f64 / 10.0) * spine_mm_per_10_pages,
            SpineConfig::Fixed { spine_width_mm } => *spine_width_mm,
        }
    }

    pub fn height_mm(&self) -> f64 {
        self.cover.height_mm
    }
}

impl CanvasConfig for CoverGeometry<'_> {
    fn page_width_mm(&self) -> f64 {
        self.spread_width_mm()
    }
    fn page_height_mm(&self) -> f64 {
        self.cover.height_mm
    }
    fn bleed_mm(&self) -> f64 {
        self.cover.bleed_mm
    }
    fn margin_mm(&self) -> f64 {
        self.cover.margin_mm
    }
    fn gap_mm(&self) -> f64 {
        self.cover.gap_mm
    }
    fn bleed_threshold_mm(&self) -> f64 {
        self.cover.bleed_threshold_mm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::{CoverMode, SpineConfig};

    fn cfg_auto(front_back: f64, spine_per_10: f64) -> CoverConfig {
        CoverConfig {
            active: true,
            mode: CoverMode::Free,
            spine_clearance_mm: 5.0,
            spine: SpineConfig::Auto {
                spine_mm_per_10_pages: spine_per_10,
            },
            front_back_width_mm: front_back,
            height_mm: 297.0,
            spine_text: None,
            bleed_mm: 3.0,
            margin_mm: 0.0,
            gap_mm: 5.0,
            bleed_threshold_mm: 3.0,
        }
    }

    fn cfg_fixed(front_back: f64, spine: f64) -> CoverConfig {
        CoverConfig {
            active: true,
            mode: CoverMode::Free,
            spine_clearance_mm: 5.0,
            spine: SpineConfig::Fixed {
                spine_width_mm: spine,
            },
            front_back_width_mm: front_back,
            height_mm: 297.0,
            spine_text: None,
            bleed_mm: 3.0,
            margin_mm: 0.0,
            gap_mm: 5.0,
            bleed_threshold_mm: 3.0,
        }
    }

    #[test]
    fn spine_width_auto() {
        let c = cfg_auto(420.0, 1.4);
        let g = CoverGeometry::new(&c, 10);
        assert!((g.spine_width_mm() - 1.4).abs() < 1e-9);
        let g100 = CoverGeometry::new(&c, 100);
        assert!((g100.spine_width_mm() - 14.0).abs() < 1e-9);
    }

    #[test]
    fn spine_width_fixed() {
        let c = cfg_fixed(420.0, 2.5);
        let g = CoverGeometry::new(&c, 10);
        assert!((g.spine_width_mm() - 2.5).abs() < 1e-9);
        let g100 = CoverGeometry::new(&c, 100);
        assert!((g100.spine_width_mm() - 2.5).abs() < 1e-9);
    }

    #[test]
    fn spread_width_auto_includes_spine() {
        let c = cfg_auto(420.0, 1.4);
        let g = CoverGeometry::new(&c, 10);
        assert!((g.spread_width_mm() - 421.4).abs() < 1e-9);
    }

    #[test]
    fn spread_width_fixed_ignores_page_count() {
        let c = cfg_fixed(420.0, 2.5);
        let g10 = CoverGeometry::new(&c, 10);
        let g100 = CoverGeometry::new(&c, 100);
        assert!((g10.spread_width_mm() - 420.0).abs() < 1e-9);
        assert!((g100.spread_width_mm() - 420.0).abs() < 1e-9);
    }

    #[test]
    fn canvas_config_uses_spread_width() {
        let c = cfg_auto(420.0, 1.4);
        let g = CoverGeometry::new(&c, 10);
        assert!((g.page_width_mm() - g.spread_width_mm()).abs() < 1e-9);
        assert!((g.page_height_mm() - 297.0).abs() < 1e-9);
    }
}
