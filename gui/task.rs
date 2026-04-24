use std::path::PathBuf;
use std::time::Duration;

use fotobuch::dto_models::ProjectState;
use fotobuch::output::typst::RenderedPage;

pub enum BackgroundTask {
    RenderPages {
        pages: Vec<usize>,
        pixel_per_pt: f32,
    },
    SwapSlots {
        src_page: usize,
        src_slot: usize,
        dst_page: usize,
        dst_slot: usize,
        pixel_per_pt: f32,
    },
    MoveSlot {
        src_page: usize,
        src_slots: Vec<usize>,
        dst_page: usize,
        pixel_per_pt: f32,
    },
    Undo {
        pixel_per_pt: f32,
    },
    Redo {
        pixel_per_pt: f32,
    },
    /// Nav-Drag: ganze Seiten tauschen.
    PageSwap {
        left: usize,
        right: usize,
        pixel_per_pt: f32,
    },
    /// Foto-Thumbnails laden (Pool-Panel).
    LoadPhotoThumbnails {
        items: Vec<(String, PathBuf)>,
    },
    /// Fotos platzieren — konkrete Zielseite oder auto-distribute.
    PlacePhotos {
        photo_ids: Vec<String>,
        dst_page: Option<usize>,
        pixel_per_pt: f32,
    },
    /// `config set key value` im Background.
    ConfigSet {
        key: String,
        value: String,
        pixel_per_pt: f32,
    },
}

#[derive(Debug)]
pub enum BackgroundResult {
    PageRendered {
        page: RenderedPage,
        /// Downsample (längste Kante ~120 px) für das Nav-Panel.
        thumb: RenderedPage,
        /// Time spent rasterising this single page.
        rasterize_duration: Duration,
        /// Time spent on `compile_document` for the task this page belongs to.
        /// All pages from the same task share the same value.
        compile_duration: Duration,
    },
    /// A command completed successfully.
    CommandDone {
        /// Updated project state, or `None` if the state did not change.
        new_state: Option<Box<ProjectState>>,
        /// Page indices that changed and need re-rendering.
        dirty_pages: Vec<usize>,
    },
    /// A command failed (user-visible error, not a render error).
    CommandFailed(String),
    Error(String),
    /// Total number of pages in the compiled Typst document (may exceed layout.len()
    /// when appendix or other extra pages are active).
    TotalPageCount(usize),
    PhotoThumbnailReady {
        id: String,
        width: u32,
        height: u32,
        /// Straight-alpha RGBA.
        pixels: Vec<u8>,
    },
}
