use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use fotobuch::output::typst::RenderedPage;

pub struct GuiState {
    pub page_textures: Vec<Option<TextureHandle>>,
    pub zoom: f32,
    /// Constant in Phase 1 — used when sending initial render task; will drive re-render in Phase 3.
    #[allow(dead_code)]
    pub base_pixel_per_pt: f32,
}

impl GuiState {
    pub fn new(num_pages: usize) -> Self {
        Self {
            page_textures: vec![None; num_pages],
            zoom: 1.0,
            base_pixel_per_pt: 1.5,
        }
    }
}

/// Uploads a rendered page into the egui texture cache.
///
/// `rendered` contains straight-alpha RGBA pixels — `from_rgba_unmultiplied` is correct here.
pub fn apply_rendered(state: &mut GuiState, ctx: &Context, rendered: RenderedPage) {
    let w = rendered.width as usize;
    let h = rendered.height as usize;
    let image = ColorImage::from_rgba_unmultiplied([w, h], &rendered.pixels);
    let handle = ctx.load_texture(
        format!("page_{}", rendered.page),
        image,
        TextureOptions::LINEAR,
    );
    state.page_textures[rendered.page] = Some(handle);
}

/// Pure zoom step: multiply by 1.1^sign(delta), clamp to [0.1, 5.0].
/// Returns `z` unchanged when `delta == 0.0`.
pub fn apply_zoom_delta(z: f32, delta: f32) -> f32 {
    if delta == 1.0 {
        return z;
    }
    (z * delta).clamp(0.1, 5.0)
}

#[cfg(test)]
mod tests {
    use fotobuch::output::typst::RenderedPage;

    use super::*;

    #[test]
    fn apply_rendered_sets_slot() {
        let ctx = egui::Context::default();
        let mut state = GuiState::new(1);
        let rendered = RenderedPage {
            page: 0,
            width: 2,
            height: 2,
            // straight-alpha white pixels
            pixels: vec![255u8; 2 * 2 * 4],
        };
        apply_rendered(&mut state, &ctx, rendered);
        assert!(state.page_textures[0].is_some());
    }

    #[test]
    fn zoom_delta() {
        assert!(apply_zoom_delta(1.0, 1.0) > 1.0, "zoom-in increases zoom");
        assert!(apply_zoom_delta(1.0, -1.0) < 1.0, "zoom-out decreases zoom");
        assert_eq!(apply_zoom_delta(4.99, 100.0), 5.0, "upper clamp");
        assert_eq!(apply_zoom_delta(0.11, -100.0), 0.1, "lower clamp");
        assert_eq!(apply_zoom_delta(1.5, 0.0), 1.5, "no-op on zero delta");
    }
}
