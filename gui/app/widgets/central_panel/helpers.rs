use egui::vec2;

pub(super) fn page_display_size(
    zoom: f32,
    page_width_mm: f64,
    page_height_mm: f64,
    bleed_mm: f64,
) -> egui::Vec2 {
    let mm_to_pt = 72.0_f32 / 25.4_f32;
    let w = (page_width_mm + 2.0 * bleed_mm) as f32 * mm_to_pt * zoom;
    let h = (page_height_mm + 2.0 * bleed_mm) as f32 * mm_to_pt * zoom;
    vec2(w, h)
}
