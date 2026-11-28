use fotobuch::models::{LayoutPage, ProjectState, Slot};

/// Fallback aspect ratio (height/width) used when no texture size is available. Approximates A4 (√2).
pub const A4_ASPECT: f32 = std::f32::consts::SQRT_2;

/// Physical dimensions and print margins for one page. Bleed extends the physical image beyond the
/// trim box; margin is the inset before the content area begins.
#[derive(Clone, Copy)]
pub struct PageDimensions {
    pub width_mm: f64,
    pub height_mm: f64,
    /// Bleed on the top and bottom edge.
    pub bleed_mm: f64,
    /// Bleed on the left edge (differs from `bleed_mm` on the spine edge of inner pages).
    pub bleed_left_mm: f64,
    /// Bleed on the right edge (differs from `bleed_mm` on the spine edge of inner pages).
    pub bleed_right_mm: f64,
    pub margin_mm: f64,
}

/// Per-axis scale factors and content-area offset derived from `PageDimensions` and a screen rect.
pub struct PageScale {
    pub scale_x: f32,
    pub scale_y: f32,
    /// Horizontal distance in mm from the physical page corner to the content-area origin:
    /// `bleed_left + margin`.
    pub offset_x_mm: f32,
    /// Vertical distance in mm from the physical page corner to the content-area origin:
    /// `bleed + margin`.
    pub offset_y_mm: f32,
}

impl PageDimensions {
    /// Dimensions of the page with layout index `page_idx`, including per-side bleed.
    pub fn for_page(project: &ProjectState, page_idx: usize) -> Self {
        let (width_mm, height_mm) = project.page_dimensions_mm(page_idx);
        let (bleed_mm, margin_mm) = project.page_bleed_margin_mm(page_idx);
        let (bleed_left_mm, bleed_right_mm) = project.page_bleed_left_right_mm(page_idx);
        Self {
            width_mm,
            height_mm,
            bleed_mm,
            bleed_left_mm,
            bleed_right_mm,
            margin_mm,
        }
    }

    /// Same bleed on all four sides.
    #[cfg(test)]
    pub fn uniform(width_mm: f64, height_mm: f64, bleed_mm: f64, margin_mm: f64) -> Self {
        Self {
            width_mm,
            height_mm,
            bleed_mm,
            bleed_left_mm: bleed_mm,
            bleed_right_mm: bleed_mm,
            margin_mm,
        }
    }

    /// Width of the rendered page image in mm (trim width plus left and right bleed).
    pub fn full_width_mm(self) -> f64 {
        self.width_mm + self.bleed_left_mm + self.bleed_right_mm
    }

    /// Height of the rendered page image in mm (trim height plus top and bottom bleed).
    pub fn full_height_mm(self) -> f64 {
        self.height_mm + 2.0 * self.bleed_mm
    }

    pub fn page_scale(self, page_rect: egui::Rect) -> PageScale {
        PageScale {
            scale_x: page_rect.width() / self.full_width_mm() as f32,
            scale_y: page_rect.height() / self.full_height_mm() as f32,
            offset_x_mm: (self.bleed_left_mm + self.margin_mm) as f32,
            offset_y_mm: (self.bleed_mm + self.margin_mm) as f32,
        }
    }
}

/// Returns `true` if two slot aspect ratios (w/h) are within 5 % of each other.
pub fn slot_ratio_similar(ratio_a: f64, ratio_b: f64) -> bool {
    if ratio_b == 0.0 {
        return false;
    }
    (ratio_a / ratio_b - 1.0).abs() < 0.0005
}

/// Maps a slot's mm coordinates to screen pixels, given the already-scaled page rect.
///
/// `page_rect` is the egui rect the page image occupies (zoom already applied).
/// The image covers `page_width + bleed_left + bleed_right` × `page_height + 2*bleed` mm.
/// Slot coordinates are in the Typst content area (offset by `bleed + margin` from the
/// physical page corner, `bleed_left + margin` horizontally).
pub fn slot_rect_on_screen(page_rect: egui::Rect, dims: PageDimensions, slot: &Slot) -> egui::Rect {
    let s = dims.page_scale(page_rect);
    let min = egui::pos2(
        page_rect.min.x + (s.offset_x_mm + slot.x_mm as f32) * s.scale_x,
        page_rect.min.y + (s.offset_y_mm + slot.y_mm as f32) * s.scale_y,
    );
    let max = egui::pos2(
        min.x + slot.width_mm as f32 * s.scale_x,
        min.y + slot.height_mm as f32 * s.scale_y,
    );
    egui::Rect::from_min_max(min, max)
}

/// Returns the index of the slot under `pos`, or `None`.
///
/// Iterates backwards so the last-drawn slot wins when slots overlap.
/// Returns `None` if `pos` is outside `page_rect`.
pub fn hit_test_slot(
    pos: egui::Pos2,
    page_rect: egui::Rect,
    layout_page: &LayoutPage,
    dims: PageDimensions,
) -> Option<usize> {
    if !page_rect.contains(pos) {
        return None;
    }
    for (idx, slot) in layout_page.slots.iter().enumerate().rev() {
        if slot_rect_on_screen(page_rect, dims, slot).contains(pos) {
            return Some(idx);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use fotobuch::models::{LayoutPage, PageMode, Slot};

    use super::*;

    fn page_rect() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(200.0, 100.0))
    }

    fn full_slot(w: f64, h: f64) -> Slot {
        Slot {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: w,
            height_mm: h,
        }
    }

    fn layout_page(slots: Vec<Slot>) -> LayoutPage {
        let photos = vec!["x".to_string(); slots.len()];
        LayoutPage {
            photos,
            slots,
            mode: PageMode::Auto,
        }
    }

    #[test]
    fn slot_at_origin_no_bleed_maps_to_page_min() {
        let slot = Slot {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: 50.0,
            height_mm: 25.0,
        };
        let dims = PageDimensions::uniform(200.0, 100.0, 0.0, 0.0);
        let rect = slot_rect_on_screen(page_rect(), dims, &slot);
        assert_eq!(rect.min, page_rect().min);
    }

    #[test]
    fn slot_offset_by_bleed_and_margin() {
        // Page 200×100mm, bleed=5mm, margin=10mm.
        // Rendered image is 210×110mm, displayed in page_rect (200×100 display).
        // Content area starts at (15mm, 15mm) from physical top-left.
        // Slot at (0,0) → screen offset = 15mm * (200px/210mm) ≈ 14.29px from page_rect.min
        let slot = Slot {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: 1.0,
            height_mm: 1.0,
        };
        let pr = page_rect(); // 200×100 px
        let dims = PageDimensions::uniform(200.0, 100.0, 5.0, 10.0);
        let rect = slot_rect_on_screen(pr, dims, &slot);
        let expected_x = pr.min.x + 15.0 * (200.0 / 210.0);
        let expected_y = pr.min.y + 15.0 * (100.0 / 110.0);
        assert!(
            (rect.min.x - expected_x).abs() < 0.01,
            "x: {} vs {}",
            rect.min.x,
            expected_x
        );
        assert!(
            (rect.min.y - expected_y).abs() < 0.01,
            "y: {} vs {}",
            rect.min.y,
            expected_y
        );
    }

    #[test]
    fn hit_test_outside_page_returns_none() {
        let lp = layout_page(vec![full_slot(200.0, 100.0)]);
        let dims = PageDimensions::uniform(200.0, 100.0, 0.0, 0.0);
        let result = hit_test_slot(
            egui::pos2(0.0, 0.0), // far outside page_rect which starts at (10, 20)
            page_rect(),
            &lp,
            dims,
        );
        assert!(result.is_none());
    }

    #[test]
    fn hit_test_hits_last_overlapping_slot_first() {
        // Two slots that both cover the full page (no bleed/margin) — second (idx 1) should win
        let lp = layout_page(vec![full_slot(200.0, 100.0), full_slot(200.0, 100.0)]);
        let center = page_rect().center();
        let dims = PageDimensions::uniform(200.0, 100.0, 0.0, 0.0);
        let result = hit_test_slot(center, page_rect(), &lp, dims);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn hit_test_between_slots_returns_none() {
        // Left slot: x=0..50mm, right slot: x=150..200mm — center (100mm) hits neither
        let left = Slot {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: 50.0,
            height_mm: 100.0,
        };
        let right = Slot {
            x_mm: 150.0,
            y_mm: 0.0,
            width_mm: 50.0,
            height_mm: 100.0,
        };
        let lp = layout_page(vec![left, right]);
        // Center of page is at x=110 (page_rect x: 10..210, center=110), y=70
        let center = page_rect().center();
        let dims = PageDimensions::uniform(200.0, 100.0, 0.0, 0.0);
        let result = hit_test_slot(center, page_rect(), &lp, dims);
        assert!(result.is_none());
    }

    #[test]
    fn slot_offset_uses_left_bleed_on_inside_edge() {
        // Right-hand inner page: spine (left) bleed 0, outside bleed 3mm.
        // Rendered image is 203×106mm, displayed 203×106 px → 1 px/mm.
        let pr = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(203.0, 106.0));
        let dims = PageDimensions {
            bleed_left_mm: 0.0,
            ..PageDimensions::uniform(200.0, 100.0, 3.0, 0.0)
        };
        let rect = slot_rect_on_screen(pr, dims, &full_slot(10.0, 10.0));
        assert!((rect.min.x - 0.0).abs() < 0.01, "x: {}", rect.min.x);
        assert!((rect.min.y - 3.0).abs() < 0.01, "y: {}", rect.min.y);
        assert!((rect.max.x - 10.0).abs() < 0.01, "max x: {}", rect.max.x);
    }
}
