use fotobuch::dto_models::{LayoutPage, Slot};

/// Returns `true` if two slot aspect ratios (w/h) are within 5 % of each other.
pub fn slot_ratio_similar(ratio_a: f64, ratio_b: f64) -> bool {
    if ratio_b == 0.0 {
        return false;
    }
    (ratio_a / ratio_b - 1.0).abs() < 0.05
}

/// Maps a slot's mm coordinates to screen pixels, given the already-scaled page rect.
///
/// `page_rect` is the egui rect the page image occupies (zoom already applied).
/// Slot coordinates are absolute to the page's top-left corner (no margin correction needed).
pub fn slot_rect_on_screen(
    page_rect: egui::Rect,
    page_width_mm: f64,
    page_height_mm: f64,
    slot: &Slot,
) -> egui::Rect {
    let scale_x = page_rect.width() / page_width_mm as f32;
    let scale_y = page_rect.height() / page_height_mm as f32;

    let min = egui::pos2(
        page_rect.min.x + slot.x_mm as f32 * scale_x,
        page_rect.min.y + slot.y_mm as f32 * scale_y,
    );
    let max = egui::pos2(
        min.x + slot.width_mm as f32 * scale_x,
        min.y + slot.height_mm as f32 * scale_y,
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
    page_width_mm: f64,
    page_height_mm: f64,
) -> Option<usize> {
    if !page_rect.contains(pos) {
        return None;
    }
    for (idx, slot) in layout_page.slots.iter().enumerate().rev() {
        let slot_rect = slot_rect_on_screen(page_rect, page_width_mm, page_height_mm, slot);
        if slot_rect.contains(pos) {
            return Some(idx);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use fotobuch::dto_models::{LayoutPage, PageMode, Slot};

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
            page: 0,
            photos,
            slots,
            mode: PageMode::Auto,
        }
    }

    #[test]
    fn slot_at_origin_maps_to_page_min() {
        let slot = Slot {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: 50.0,
            height_mm: 25.0,
        };
        let rect = slot_rect_on_screen(page_rect(), 200.0, 100.0, &slot);
        assert_eq!(rect.min, page_rect().min);
    }

    #[test]
    fn slot_center_scales_linearly() {
        // Slot at center: x=100mm, y=50mm on a 200×100mm page → center of page_rect
        let slot = Slot {
            x_mm: 100.0,
            y_mm: 50.0,
            width_mm: 10.0,
            height_mm: 10.0,
        };
        let pr = page_rect();
        let rect = slot_rect_on_screen(pr, 200.0, 100.0, &slot);
        assert!((rect.min.x - pr.center().x).abs() < 1e-3);
        assert!((rect.min.y - pr.center().y).abs() < 1e-3);
    }

    #[test]
    fn full_page_slot_matches_page_rect() {
        let slot = full_slot(200.0, 100.0);
        let pr = page_rect();
        let rect = slot_rect_on_screen(pr, 200.0, 100.0, &slot);
        assert!((rect.min.x - pr.min.x).abs() < 1e-3);
        assert!((rect.min.y - pr.min.y).abs() < 1e-3);
        assert!((rect.max.x - pr.max.x).abs() < 1e-3);
        assert!((rect.max.y - pr.max.y).abs() < 1e-3);
    }

    #[test]
    fn hit_test_outside_page_returns_none() {
        let lp = layout_page(vec![full_slot(200.0, 100.0)]);
        let result = hit_test_slot(
            egui::pos2(0.0, 0.0), // far outside page_rect which starts at (10, 20)
            page_rect(),
            &lp,
            200.0,
            100.0,
        );
        assert!(result.is_none());
    }

    #[test]
    fn hit_test_hits_last_overlapping_slot_first() {
        // Two slots that both cover the full page — second (idx 1) should win
        let lp = layout_page(vec![full_slot(200.0, 100.0), full_slot(200.0, 100.0)]);
        let center = page_rect().center();
        let result = hit_test_slot(center, page_rect(), &lp, 200.0, 100.0);
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
        let result = hit_test_slot(center, page_rect(), &lp, 200.0, 100.0);
        assert!(result.is_none());
    }
}
