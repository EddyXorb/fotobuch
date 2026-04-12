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
        src_slot: usize,
        dst_page: usize,
        pixel_per_pt: f32,
    },
    Undo {
        pixel_per_pt: f32,
    },
    Redo {
        pixel_per_pt: f32,
    },
}

#[derive(Debug)]
pub enum BackgroundResult {
    PageRendered {
        page: RenderedPage,
        /// Time spent rasterising this single page.
        rasterize_duration: Duration,
        /// Time spent on `compile_document` for the task this page belongs to.
        /// All pages from the same task share the same value.
        compile_duration: Duration,
    },
    /// A command completed successfully.
    CommandDone {
        new_state: Box<ProjectState>,
        /// Page indices that changed and need re-rendering.
        dirty_pages: Vec<usize>,
    },
    /// A command failed (user-visible error, not a render error).
    CommandFailed(String),
    Error(String),
}
