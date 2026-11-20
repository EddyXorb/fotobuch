use egui::vec2;

use super::super::geometry::PageDimensions;

pub(super) fn page_display_size(zoom: f32, dims: PageDimensions) -> egui::Vec2 {
    let mm_to_pt = 72.0_f32 / 25.4_f32;
    let w = dims.full_width_mm() as f32 * mm_to_pt * zoom;
    let h = dims.full_height_mm() as f32 * mm_to_pt * zoom;
    vec2(w, h)
}
