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
    /// Time spent handling input last frame.
    pub input_handlers: Duration,
    /// Time spent drawing pages (scroll area) last frame.
    pub show_panels: Duration,
    /// Render duration per page, sorted by page index.
    pub render_pages: Vec<(usize, Duration)>,
    /// Time for the last `compile_document` call in the background worker.
    pub typst_compile: Duration,
    /// Average rasterisation time per page across all received pages.
    pub typst_rasterize_avg: Duration,

    pub frame_cnt: usize,
}

impl Timings {
    pub fn record_render(
        &mut self,
        page: usize,
        rasterize_duration: Duration,
        compile_duration: Duration,
    ) {
        match self.render_pages.iter_mut().find(|(p, _)| *p == page) {
            Some(entry) => entry.1 = rasterize_duration,
            None => {
                self.render_pages.push((page, rasterize_duration));
                self.render_pages.sort_by_key(|(p, _)| *p);
            }
        }
        self.typst_compile = compile_duration;
        // Recompute average over all known per-page rasterize times.
        if !self.render_pages.is_empty() {
            let total: Duration = self.render_pages.iter().map(|(_, d)| *d).sum();
            self.typst_rasterize_avg = total / self.render_pages.len() as u32;
        }
    }
}
