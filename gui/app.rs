mod overlay;
mod view;

use std::time::Instant;

use crossbeam::channel::{Receiver, Sender};

use crate::state::{self, GuiState};
use crate::task::{BackgroundResult, BackgroundTask};

pub struct FotobuchApp {
    state: GuiState,
    // held to keep the background thread alive; will be used for re-render in Phase 3
    #[allow(dead_code)]
    task_tx: Sender<BackgroundTask>,
    result_rx: Receiver<BackgroundResult>,
    page_width_mm: f64,
    page_height_mm: f64,
}

impl FotobuchApp {
    pub fn new(
        _cc: &eframe::CreationContext,
        state: GuiState,
        task_tx: Sender<BackgroundTask>,
        result_rx: Receiver<BackgroundResult>,
        page_width_mm: f64,
        page_height_mm: f64,
    ) -> Self {
        Self {
            state,
            task_tx,
            result_rx,
            page_width_mm,
            page_height_mm,
        }
    }

    fn drain_results(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.result_rx.try_recv() {
            match msg {
                BackgroundResult::PageRendered { page: r, duration } => {
                    let page_idx = r.page;
                    state::apply_rendered(&mut self.state, ctx, r);
                    self.state.timings.record_render(page_idx, duration);
                }
                BackgroundResult::Error(e) => {
                    tracing::error!(%e, "render error");
                }
            }
        }
    }

    fn request_repaint_if_loading(&self, ctx: &egui::Context) {
        if self.state.page_textures.iter().any(Option::is_none) {
            ctx.request_repaint();
        }
    }

    fn apply_zoom(&mut self, ctx: &egui::Context) {
        let delta = ctx.input(|i| {
            if i.modifiers.ctrl {
                i.zoom_delta()
            } else {
                1.0
            }
        });
        if delta != 1.0 {
            self.state.zoom = state::apply_zoom_delta(self.state.zoom, delta);
        }
    }

    fn show_pages(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    for i in 0..self.state.page_textures.len() {
                        view::draw_page(
                            ui,
                            &self.state,
                            i,
                            self.page_width_mm,
                            self.page_height_mm,
                        );
                    }
                });
            });
    }
}

impl eframe::App for FotobuchApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let t_frame = Instant::now();
        let ctx = ui.ctx().clone();

        // consume_key ensures the toggle fires exactly once per key press,
        // even if input() is called multiple times within the same frame.
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F2)) {
            self.state.timings.show = !self.state.timings.show;
        }

        let t = Instant::now();
        self.drain_results(&ctx);
        self.state.timings.drain_results = t.elapsed();

        self.request_repaint_if_loading(&ctx);

        let t = Instant::now();
        self.apply_zoom(&ctx);
        self.state.timings.apply_zoom = t.elapsed();

        let t = Instant::now();
        self.show_pages(ui);
        self.state.timings.show_pages = t.elapsed();

        self.state.timings.ui_frame = t_frame.elapsed();

        if self.state.timings.show {
            overlay::show_timings_overlay(&self.state.timings, &ctx);
        }
    }
}
