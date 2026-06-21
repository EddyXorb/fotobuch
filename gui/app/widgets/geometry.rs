use fotobuch::models::{LayoutPage, Slot};

/// Fallback aspect ratio (height/width) used when no texture size is available. Approximates A4 (√2).
pub const A4_ASPECT: f32 = std::f32::consts::SQRT_2;

/// Physical dimensions and print margins for one page. Bleed extends the physical image beyond the
/// trim box; margin is the inset before the content area begins.
#[derive(Clone, Copy)]
pub struct PageDimensions {
    pub width_mm: f64,
    pub height_mm: f64,
    pub bleed_mm: f64,
    pub margin_mm: f64,
}

/// Per-axis scale factors and content-area offset derived from `PageDimensions` and a screen rect.
pub struct PageScale {
    pub scale_x: f32,
    pub scale_y: f32,
    /// Distance in mm (converted to screen pixels via scale) from the physical page corner to the
    /// content-area origin: `bleed + margin`.
    pub offset_mm: f32,
}

impl PageDimensions {
    pub fn page_scale(self, page_rect: egui::Rect) -> PageScale {
        let full_w = (self.width_mm + 2.0 * self.bleed_mm) as f32;
        let full_h = (self.height_mm + 2.0 * self.bleed_mm) as f32;
        PageScale {
            scale_x: page_rect.width() / full_w,
            scale_y: page_rect.height() / full_h,
            offset_mm: (self.bleed_mm + self.margin_mm) as f32,
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
/// The image covers `page_width + 2*bleed` × `page_height + 2*bleed` mm.
/// Slot coordinates are in the Typst content area (offset by `bleed + margin` from the
/// physical page corner).
pub fn slot_rect_on_screen(page_rect: egui::Rect, dims: PageDimensions, slot: &Slot) -> egui::Rect {
    let s = dims.page_scale(page_rect);
    let min = egui::pos2(
        page_rect.min.x + (s.offset_mm + slot.x_mm as f32) * s.scale_x,
        page_rect.min.y + (s.offset_mm + slot.y_mm as f32) * s.scale_y,
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
        let dims = PageDimensions {
            width_mm: 200.0,
            height_mm: 100.0,
            bleed_mm: 0.0,
            margin_mm: 0.0,
        };
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
        let dims = PageDimensions {
            width_mm: 200.0,
            height_mm: 100.0,
            bleed_mm: 5.0,
            margin_mm: 10.0,
        };
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
        let dims = PageDimensions {
            width_mm: 200.0,
            height_mm: 100.0,
            bleed_mm: 0.0,
            margin_mm: 0.0,
        };
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
        let dims = PageDimensions {
            width_mm: 200.0,
            height_mm: 100.0,
            bleed_mm: 0.0,
            margin_mm: 0.0,
        };
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
        let dims = PageDimensions {
            width_mm: 200.0,
            height_mm: 100.0,
            bleed_mm: 0.0,
            margin_mm: 0.0,
        };
        let result = hit_test_slot(center, page_rect(), &lp, dims);
        assert!(result.is_none());
    }
}
