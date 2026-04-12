mod derived;
mod selection;
mod timings;

pub use derived::DerivedState;
pub use selection::Selection;
pub use timings::Timings;

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use fotobuch::dto_models::ProjectState;
use fotobuch::output::typst::RenderedPage;

pub struct GuiState {
    pub project_state: ProjectState,
    pub derived: DerivedState,
    pub page_textures: Vec<Option<TextureHandle>>,
    pub zoom: f32,
    /// Constant in Phase 2 — will drive re-render in Phase 3.
    #[allow(dead_code)]
    pub base_pixel_per_pt: f32,
    pub selection: Selection,
    /// `(page_idx, slot_idx)` hovered in the previous frame.
    pub hovered_slot: Option<(usize, usize)>,
    pub timings: Timings,
}

impl GuiState {
    pub fn new(project_state: ProjectState) -> Self {
        let num_pages = project_state.layout.len();
        let derived = DerivedState::rebuild(&project_state);
        Self {
            project_state,
            derived,
            page_textures: vec![None; num_pages],
            zoom: 1.0,
            base_pixel_per_pt: 1.5,
            selection: Selection::None,
            hovered_slot: None,
            timings: Timings::default(),
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

/// Pure zoom step: `z * delta`, clamped to [0.1, 5.0].
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

    fn minimal_project() -> ProjectState {
        ProjectState::default()
    }

    #[test]
    fn new_derives_num_pages() {
        let state = GuiState::new(minimal_project());
        assert_eq!(state.page_textures.len(), 0);
    }

    #[test]
    fn apply_rendered_sets_slot() {
        let ctx = egui::Context::default();
        let mut project = minimal_project();
        project.layout = vec![fotobuch::dto_models::LayoutPage {
            page: 0,
            photos: vec![],
            slots: vec![],
            mode: fotobuch::dto_models::PageMode::Auto,
        }];
        let mut state = GuiState::new(project);
        let rendered = RenderedPage {
            page: 0,
            width: 2,
            height: 2,
            pixels: vec![255u8; 2 * 2 * 4],
        };
        apply_rendered(&mut state, &ctx, rendered);
        assert!(state.page_textures[0].is_some());
    }

    #[test]
    fn zoom_delta() {
        assert!(apply_zoom_delta(1.0, 1.1) > 1.0, "zoom-in increases zoom");
        assert!(apply_zoom_delta(1.0, 0.9) < 1.0, "zoom-out decreases zoom");
        assert_eq!(apply_zoom_delta(4.9, 2.0), 5.0, "upper clamp");
        assert_eq!(apply_zoom_delta(0.15, 0.1), 0.1, "lower clamp");
        assert_eq!(apply_zoom_delta(1.5, 1.0), 1.5, "no-op on 1.0 delta");
    }
}
