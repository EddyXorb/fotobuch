mod draw_page;
mod geometry;
mod input_handler;
mod overlay;
mod statusbar;
mod toolbar;
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
}

impl FotobuchApp {
    pub fn new(
        _cc: &eframe::CreationContext,
        state: GuiState,
        task_tx: Sender<BackgroundTask>,
        result_rx: Receiver<BackgroundResult>,
    ) -> Self {
        Self {
            state,
            task_tx,
            result_rx,
        }
    }

    fn drain_results(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.result_rx.try_recv() {
            match msg {
                BackgroundResult::PageRendered {
                    page: r,
                    rasterize_duration,
                    compile_duration,
                } => {
                    let page_idx = r.page;
                    state::apply_rendered(&mut self.state, ctx, r);
                    self.state.timings.record_render(
                        page_idx,
                        rasterize_duration,
                        compile_duration,
                    );
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

    fn show_pages(&mut self, ui: &mut egui::Ui) {
        let num_pages = self.state.project_state.layout.len();
        let mut new_hovered: Option<(usize, usize)> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    for i in 0..num_pages {
                        if let Some(slot_idx) = view::draw_page(ui, &self.state, i)
                            && new_hovered.is_none()
                        {
                            new_hovered = Some((i, slot_idx));
                        }
                    }
                });
            });

        self.state.hovered_slot = new_hovered;
    }
}

impl eframe::App for FotobuchApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let t_frame = Instant::now();
        let ctx = ui.ctx().clone();

        let t = Instant::now();
        self.drain_results(&ctx);
        self.state.timings.drain_results = t.elapsed();

        self.request_repaint_if_loading(&ctx);

        let t = Instant::now();
        input_handler::handle(&mut self.state, &ctx);
        self.state.timings.apply_zoom = t.elapsed();

        egui::Panel::top("toolbar").show_inside(ui, toolbar::show);

        egui::Panel::bottom("statusbar").show_inside(ui, |ui| statusbar::show(ui, &self.state));

        let t = Instant::now();
        egui::CentralPanel::default().show_inside(ui, |ui| self.show_pages(ui));
        self.state.timings.show_pages = t.elapsed();

        self.state.timings.ui_frame = t_frame.elapsed();

        if self.state.timings.show {
            overlay::show_timings_overlay(&self.state.timings, &ctx);
        }
    }
}
