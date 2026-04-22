use egui::vec2;

use super::super::geometry::PageDimensions;

pub(super) fn page_display_size(zoom: f32, dims: PageDimensions) -> egui::Vec2 {
    let mm_to_pt = 72.0_f32 / 25.4_f32;
    let w = (dims.width_mm + 2.0 * dims.bleed_mm) as f32 * mm_to_pt * zoom;
    let h = (dims.height_mm + 2.0 * dims.bleed_mm) as f32 * mm_to_pt * zoom;
    vec2(w, h)
}
