use std::time::Duration;

/// Per-frame timing measurements for the UI thread and background renders.
#[derive(Default)]
pub struct Timings {
    /// Whether the timing overlay (F2) is visible.
    pub show: bool,
    /// Total time spent in `FotobuchApp::ui()` last frame.
    pub ui_frame: Duration,
    /// Time spent draining background results last frame.
    pub drain_results: Duration,
    /// Time spent applying zoom input last frame.
    pub apply_zoom: Duration,
    /// Time spent drawing pages (scroll area) last frame.
    pub show_pages: Duration,
    /// Render duration per page, sorted by page index.
    /// Updated as pages arrive from the background thread.
    pub render_pages: Vec<(usize, Duration)>,
}

impl Timings {
    pub fn record_render(&mut self, page: usize, duration: Duration) {
        match self.render_pages.iter_mut().find(|(p, _)| *p == page) {
            Some(entry) => entry.1 = duration,
            None => {
                self.render_pages.push((page, duration));
                self.render_pages.sort_by_key(|(p, _)| *p);
            }
        }
    }
}
