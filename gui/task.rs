use fotobuch::output::typst::RenderedPage;

pub enum BackgroundTask {
    RenderPages {
        pages: Vec<usize>,
        pixel_per_pt: f32,
    },
}

pub enum BackgroundResult {
    PageRendered(RenderedPage),
    Error(String),
}
