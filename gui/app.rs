use crossbeam::channel::{Receiver, Sender};
use egui::vec2;

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
                BackgroundResult::PageRendered(r) => {
                    state::apply_rendered(&mut self.state, ctx, r);
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
                        draw_page(ui, &self.state, i, self.page_width_mm, self.page_height_mm);
                    }
                });
            });
    }
}

impl eframe::App for FotobuchApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.drain_results(&ctx);
        self.request_repaint_if_loading(&ctx);
        self.apply_zoom(&ctx);
        self.show_pages(ui);
    }
}

fn draw_page(
    ui: &mut egui::Ui,
    state: &GuiState,
    i: usize,
    page_width_mm: f64,
    page_height_mm: f64,
) {
    // 0-based page label
    ui.label(format!("Seite {i}"));

    let mm_to_pt = 72.0_f32 / 25.4_f32;
    let base_w = page_width_mm as f32 * mm_to_pt;
    let base_h = page_height_mm as f32 * mm_to_pt;
    let size = vec2(base_w * state.zoom, base_h * state.zoom);

    if let Some(tex) = &state.page_textures[i] {
        ui.add(egui::Image::from_texture(tex).fit_to_exact_size(size));
    } else {
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_gray(200));
    }
}
