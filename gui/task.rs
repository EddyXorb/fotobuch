use std::time::Duration;

use fotobuch::output::typst::RenderedPage;

pub enum BackgroundTask {
    RenderPages {
        pages: Vec<usize>,
        pixel_per_pt: f32,
    },
}

pub enum BackgroundResult {
    PageRendered {
        page: RenderedPage,
        /// Time spent rasterising this single page.
        rasterize_duration: Duration,
        /// Time spent on `compile_document` for the task this page belongs to.
        /// All pages from the same task share the same value.
        compile_duration: Duration,
    },
    Error(String),
}
