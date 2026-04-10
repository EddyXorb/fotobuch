use egui::vec2;

use crate::state::GuiState;

pub fn draw_page(
    ui: &mut egui::Ui,
    state: &GuiState,
    i: usize,
    page_width_mm: f64,
    page_height_mm: f64,
) {
    // 0-based page label
    ui.label(format!("Seite {i}"));

    let mm_to_pt = 72.0_f32 / 25.4_f32;
    let base_w = page_width_mm as f32 * mm_to_pt;
    let base_h = page_height_mm as f32 * mm_to_pt;
    let size = vec2(base_w * state.zoom, base_h * state.zoom);

    if let Some(tex) = &state.page_textures[i] {
        ui.add(egui::Image::from_texture(tex).fit_to_exact_size(size));
    } else {
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_gray(200));
    }
}
