use egui::vec2;

use crate::state::GuiState;

use super::geometry;

/// Draw a single page at index `i`.
///
/// Returns the slot index the pointer is hovering over (if any).
/// Overlays are drawn with the previous frame's `hovered_slot` and `selection` — one-frame lag,
/// standard egui practice.
pub fn draw_page(ui: &mut egui::Ui, state: &GuiState, page_idx: usize) -> Option<usize> {
    ui.label(format!("Page {page_idx}"));

    let (page_width_mm, page_height_mm) = state.project_state.page_dimensions_mm(page_idx);
    let size = page_display_size(state.zoom, page_width_mm, page_height_mm);
    let page_rect = render_page_image(ui, state, page_idx, size);

    if let Some(layout_page) = state.project_state.layout.get(page_idx) {
        draw_slot_overlays(
            ui,
            page_rect,
            state,
            page_idx,
            page_width_mm,
            page_height_mm,
        );
        hit_test_pointer(ui, page_rect, layout_page, page_width_mm, page_height_mm)
    } else {
        None
    }
}

/// Computes the on-screen size of a page in egui points.
fn page_display_size(zoom: f32, page_width_mm: f64, page_height_mm: f64) -> egui::Vec2 {
    let mm_to_pt = 72.0_f32 / 25.4_f32;
    vec2(
        page_width_mm as f32 * mm_to_pt * zoom,
        page_height_mm as f32 * mm_to_pt * zoom,
    )
}

/// Renders the page texture or a grey placeholder. Returns the allocated page rect.
fn render_page_image(
    ui: &mut egui::Ui,
    state: &GuiState,
    page_idx: usize,
    size: egui::Vec2,
) -> egui::Rect {
    if let Some(tex) = &state.page_textures[page_idx] {
        ui.add(egui::Image::from_texture(tex).fit_to_exact_size(size))
            .rect
    } else {
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_gray(200));
        rect
    }
}

/// Paints hover (blue fill) and selection (green stroke) overlays for each slot.
fn draw_slot_overlays(
    ui: &mut egui::Ui,
    page_rect: egui::Rect,
    state: &GuiState,
    page_idx: usize,
    page_width_mm: f64,
    page_height_mm: f64,
) {
    let layout_page = match state.project_state.layout.get(page_idx) {
        Some(lp) => lp,
        None => return,
    };

    let painter = ui.painter();
    for (slot_idx, slot) in layout_page.slots.iter().enumerate() {
        let slot_rect =
            geometry::slot_rect_on_screen(page_rect, page_width_mm, page_height_mm, slot);

        if state.hovered_slot == Some((page_idx, slot_idx)) {
            painter.rect_filled(
                slot_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 120, 255, 38),
            );
        }
        if state.selection.is_selected(page_idx, slot_idx) {
            painter.rect_stroke(
                slot_rect,
                0.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(50, 200, 80)),
                egui::StrokeKind::Middle,
            );
        }
    }
}

/// Hit-tests the current pointer position against the page's slots. Returns the slot index or
/// `None` when the pointer is outside the page or between slots.
fn hit_test_pointer(
    ui: &mut egui::Ui,
    page_rect: egui::Rect,
    layout_page: &fotobuch::dto_models::LayoutPage,
    page_width_mm: f64,
    page_height_mm: f64,
) -> Option<usize> {
    ui.ctx().pointer_hover_pos().and_then(|pos| {
        geometry::hit_test_slot(pos, page_rect, layout_page, page_width_mm, page_height_mm)
    })
}
