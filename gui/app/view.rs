use egui::vec2;

use crate::state::GuiState;

use super::geometry;

/// Draw a single page at index `i` and return the slot index that the pointer is
/// hovering over (if any), using the pointer position from the current frame.
///
/// Overlays are drawn with the previous frame's `hovered_slot` and `selection`
/// so there is a one-frame lag — standard egui practice.
pub fn draw_page(ui: &mut egui::Ui, state: &GuiState, i: usize) -> Option<usize> {
    ui.label(format!("Seite {i}"));

    let page_width_mm = state.project_state.config.book.page_width_mm;
    let page_height_mm = state.project_state.config.book.page_height_mm;

    let mm_to_pt = 72.0_f32 / 25.4_f32;
    let base_w = page_width_mm as f32 * mm_to_pt;
    let base_h = page_height_mm as f32 * mm_to_pt;
    let size = vec2(base_w * state.zoom, base_h * state.zoom);

    let page_rect = if let Some(tex) = &state.page_textures[i] {
        ui.add(egui::Image::from_texture(tex).fit_to_exact_size(size))
            .rect
    } else {
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_gray(200));
        rect
    };

    // Draw slot overlays using previous-frame hover/selection.
    if let Some(layout_page) = state.project_state.layout.get(i) {
        let painter = ui.painter();
        for (slot_idx, slot) in layout_page.slots.iter().enumerate() {
            let slot_rect =
                geometry::slot_rect_on_screen(page_rect, page_width_mm, page_height_mm, slot);

            if state.hovered_slot == Some((i, slot_idx)) {
                painter.rect_filled(
                    slot_rect,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(0, 120, 255, 38),
                );
            }
            if state.selection.is_selected(i, slot_idx) {
                painter.rect_stroke(
                    slot_rect,
                    0.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(50, 200, 80)),
                    egui::StrokeKind::Middle,
                );
            }
        }

        // Hit-test with current pointer position.
        ui.ctx().pointer_hover_pos().and_then(|pos| {
            geometry::hit_test_slot(pos, page_rect, layout_page, page_width_mm, page_height_mm)
        })
    } else {
        None
    }
}
