mod config_panel;
mod derived;
mod drag;
mod hover;
mod pool;
mod selection;
mod selections;
mod thumb;
mod timings;

pub use config_panel::ConfigPanelState;
pub use derived::DerivedState;
pub use drag::{DragMode, DragSource, DragState};
pub use hover::HoveredTarget;
pub use pool::PhotoSelection;
pub use selection::SlotSelection;
pub use selections::Selections;
pub use thumb::PhotoThumbState;
pub use timings::Timings;

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use fotobuch::dto_models::ProjectState;
use fotobuch::output::typst::RenderedPage;

/// Scroll and zoom-correction state for the central panel. Updated each frame by draw_pages,
/// read by handle_zoom to compute corrected offsets.
#[derive(Default)]
pub struct CentralScrollState {
    pub scroll_y: f32,
    pub viewport_top: f32,
    pub pending_scroll_y: Option<f32>,
}

pub struct GuiState {
    pub project_state: ProjectState,
    pub derived: DerivedState,
    pub page_textures: Vec<Option<TextureHandle>>,
    pub page_dirty: Vec<bool>,
    pub page_thumb_textures: Vec<Option<TextureHandle>>,
    pub zoom: f32,
    pub base_pixel_per_pt: f32,
    pub selections: Selections,
    pub hovered: Option<HoveredTarget>,
    pub drag: DragState,
    pub drag_mode: DragMode,
    pub thumb: PhotoThumbState,
    pub scroll_to_page: Option<usize>,
    pub config_panel: ConfigPanelState,
    pub timings: Timings,
    pub central_scroll: CentralScrollState,
}

impl GuiState {
    pub fn new(project_state: ProjectState) -> Self {
        let num_pages = project_state.layout.len();
        let derived = DerivedState::rebuild(&project_state);
        Self {
            project_state,
            derived,
            page_textures: vec![None; num_pages],
            page_dirty: vec![false; num_pages],
            page_thumb_textures: vec![None; num_pages],
            zoom: 1.0,
            base_pixel_per_pt: 1.5,
            selections: Selections::default(),
            hovered: None,
            drag: DragState::Idle,
            drag_mode: DragMode::Swap,
            thumb: PhotoThumbState::default(),
            scroll_to_page: None,
            config_panel: ConfigPanelState::default(),
            timings: Timings::default(),
            central_scroll: CentralScrollState::default(),
        }
    }
}

/// Uploads a rendered page (full + thumb) into the egui texture cache and clears its dirty flag.
pub fn apply_rendered(
    state: &mut GuiState,
    ctx: &Context,
    full: RenderedPage,
    thumb: RenderedPage,
) {
    let page = full.page;
    let img = ColorImage::from_rgba_unmultiplied([full.width as _, full.height as _], &full.pixels);
    let handle = ctx.load_texture(format!("page_{page}"), img, TextureOptions::LINEAR);
    if let Some(slot) = state.page_textures.get_mut(page) {
        *slot = Some(handle);
    }
    if let Some(d) = state.page_dirty.get_mut(page) {
        *d = false;
    }

    let thumb_img =
        ColorImage::from_rgba_unmultiplied([thumb.width as _, thumb.height as _], &thumb.pixels);
    let thumb_tex = ctx.load_texture(
        format!("page_thumb_{page}"),
        thumb_img,
        TextureOptions::LINEAR,
    );
    if let Some(s) = state.page_thumb_textures.get_mut(page) {
        *s = Some(thumb_tex);
    }
}

/// Pure zoom step: `z * delta`, clamped to [0.1, 5.0].
pub fn apply_zoom_delta(z: f32, delta: f32) -> f32 {
    if delta == 1.0 {
        return z;
    }
    (z * delta).clamp(0.1, 5.0)
}

/// Resize `page_textures`, `page_dirty` and `page_thumb_textures` to match a new page count.
pub fn resize_page_vecs(state: &mut GuiState, new_len: usize) {
    state.page_textures.resize(new_len, None);
    state.page_dirty.resize(new_len, false);
    state.page_thumb_textures.resize(new_len, None);
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
        assert_eq!(state.page_dirty.len(), 0);
    }

    #[test]
    fn apply_rendered_sets_slot_and_clears_dirty() {
        let ctx = egui::Context::default();
        let mut project = minimal_project();
        project.layout = vec![fotobuch::dto_models::LayoutPage {
            page: 0,
            photos: vec![],
            slots: vec![],
            mode: fotobuch::dto_models::PageMode::Auto,
        }];
        let mut state = GuiState::new(project);
        state.page_dirty[0] = true;
        let full = RenderedPage {
            page: 0,
            width: 2,
            height: 2,
            pixels: vec![255u8; 16],
        };
        let thumb = RenderedPage {
            page: 0,
            width: 1,
            height: 1,
            pixels: vec![255u8; 4],
        };
        apply_rendered(&mut state, &ctx, full, thumb);
        assert!(state.page_textures[0].is_some());
        assert!(state.page_thumb_textures[0].is_some());
        assert!(!state.page_dirty[0]);
    }

    #[test]
    fn resize_page_vecs_also_resizes_thumb_vec() {
        let mut state = GuiState::new(minimal_project());
        resize_page_vecs(&mut state, 4);
        assert_eq!(state.page_thumb_textures.len(), 4);
        resize_page_vecs(&mut state, 2);
        assert_eq!(state.page_thumb_textures.len(), 2);
    }

    #[test]
    fn zoom_delta() {
        assert!(apply_zoom_delta(1.0, 1.1) > 1.0, "zoom-in increases zoom");
        assert!(apply_zoom_delta(1.0, 0.9) < 1.0, "zoom-out decreases zoom");
        assert_eq!(apply_zoom_delta(4.9, 2.0), 5.0, "upper clamp");
        assert_eq!(apply_zoom_delta(0.15, 0.1), 0.1, "lower clamp");
        assert_eq!(apply_zoom_delta(1.5, 1.0), 1.5, "no-op on 1.0 delta");
    }

    #[test]
    fn resize_page_vecs_grows_and_shrinks() {
        let mut state = GuiState::new(minimal_project());
        resize_page_vecs(&mut state, 3);
        assert_eq!(state.page_textures.len(), 3);
        assert_eq!(state.page_dirty.len(), 3);
        resize_page_vecs(&mut state, 1);
        assert_eq!(state.page_textures.len(), 1);
    }
}
