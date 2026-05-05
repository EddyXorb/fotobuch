mod add_dialog;
mod config_panel;
mod context_menu;
mod derived;
mod drag;
mod hover;
mod multi_selection;
mod page_cache;
mod pool;
mod selection;
mod selections;
mod timings;
mod toasts;
mod viewport;
mod weight_slider;

pub use add_dialog::AddDialogState;
pub use config_panel::ConfigPanelState;
pub use context_menu::ContextMenu;
pub use derived::DerivedState;
pub use drag::{ActiveDrag, DragMode, DragSource, DragState};
pub use hover::HoveredTarget;
pub use multi_selection::MultiSelection;
pub use page_cache::PageCache;
pub use pool::PhotoSelection;
pub use selection::SlotSelection;
pub use selections::Selections;
pub use timings::Timings;
pub use toasts::ToastQueue;
pub use viewport::Viewport;
pub use weight_slider::WeightSlider;

use std::collections::HashMap;

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use fotobuch::dto_models::ProjectState;
use fotobuch::output::typst::RenderedPage;

pub struct DataState {
    pub project: ProjectState,
    pub derived: DerivedState,
    pub pages: PageCache,
    pub thumbs: HashMap<String, TextureHandle>,
    pub timings: Timings,
    pub toasts: ToastQueue,
}

pub struct InteractionState {
    pub selections: Selections,
    pub hovered: Option<HoveredTarget>,
    pub drag: DragState,
    pub viewport: Viewport,
    pub config: ConfigPanelState,
    pub goto_open: bool,
    pub rebuild_all_confirm: bool,
    pub add_dialog: AddDialogState,
    pub context_menu: Option<ContextMenu>,
    pub weight_slider: WeightSlider,
}

pub struct GuiState {
    pub data: DataState,
    pub interaction: InteractionState,
}

impl GuiState {
    pub fn new(project: ProjectState) -> Self {
        let num_pages = project.layout.len();
        let derived = DerivedState::rebuild(&project);
        Self {
            data: DataState {
                project,
                derived,
                pages: PageCache::new(num_pages),
                thumbs: HashMap::new(),
                timings: Timings::default(),
                toasts: ToastQueue::default(),
            },
            interaction: InteractionState {
                selections: Selections::default(),
                hovered: None,
                drag: DragState::default(),
                viewport: Viewport::default(),
                config: ConfigPanelState::default(),
                goto_open: false,
                rebuild_all_confirm: false,
                add_dialog: AddDialogState::default(),
                context_menu: None,
                weight_slider: WeightSlider::default(),
            },
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
    if let Some(slot) = state.data.pages.textures.get_mut(page) {
        *slot = Some(handle);
    }
    if let Some(d) = state.data.pages.dirty.get_mut(page) {
        *d = false;
    }

    let thumb_img =
        ColorImage::from_rgba_unmultiplied([thumb.width as _, thumb.height as _], &thumb.pixels);
    let thumb_tex = ctx.load_texture(
        format!("page_thumb_{page}"),
        thumb_img,
        TextureOptions::LINEAR,
    );
    if let Some(s) = state.data.pages.thumb_textures.get_mut(page) {
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

/// Resize all page cache vecs to match a new page count.
pub fn resize_page_vecs(state: &mut GuiState, new_len: usize) {
    state.data.pages.resize(new_len);
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
        assert_eq!(state.data.pages.textures.len(), 0);
        assert_eq!(state.data.pages.dirty.len(), 0);
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
        state.data.pages.dirty[0] = true;
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
        assert!(state.data.pages.textures[0].is_some());
        assert!(state.data.pages.thumb_textures[0].is_some());
        assert!(!state.data.pages.dirty[0]);
    }

    #[test]
    fn resize_page_vecs_also_resizes_thumb_vec() {
        let mut state = GuiState::new(minimal_project());
        resize_page_vecs(&mut state, 4);
        assert_eq!(state.data.pages.thumb_textures.len(), 4);
        resize_page_vecs(&mut state, 2);
        assert_eq!(state.data.pages.thumb_textures.len(), 2);
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
        assert_eq!(state.data.pages.textures.len(), 3);
        assert_eq!(state.data.pages.dirty.len(), 3);
        resize_page_vecs(&mut state, 1);
        assert_eq!(state.data.pages.textures.len(), 1);
    }
}
