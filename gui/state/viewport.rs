/// Scroll/zoom state for the central panel.
#[derive(Default)]
pub struct ScrollState {
    pub scroll_y: f32,
    pub viewport_top: f32,
    pub pending_scroll_y: Option<f32>,
}

/// View parameters for the central panel: zoom level, base DPI scale, scroll, and nav scroll target.
pub struct Viewport {
    pub zoom: f32,
    pub pixel_per_pt: f32,
    pub scroll: ScrollState,
    pub scroll_to_page: Option<usize>,
    /// When `true`, zoom will be adjusted on the next frame to fit the widest page.
    pub fit_pending: bool,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pixel_per_pt: 1.5,
            scroll: ScrollState::default(),
            scroll_to_page: None,
            fit_pending: true,
        }
    }
}
