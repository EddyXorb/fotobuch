mod geometry;
mod overlay;
mod statusbar;
mod toolbar;
mod view;

use std::time::Instant;

use crossbeam::channel::{Receiver, Sender};

use crate::state::{self, GuiState, Selection};
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

    fn handle_input(&mut self, ctx: &egui::Context) {
        // F2: toggle timings overlay
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F2)) {
            self.state.timings.show = !self.state.timings.show;
        }

        // Ctrl+scroll: zoom
        let zoom_delta = ctx.input(|i| {
            if i.modifiers.ctrl {
                i.zoom_delta()
            } else {
                1.0
            }
        });
        if zoom_delta != 1.0 {
            self.state.zoom = state::apply_zoom_delta(self.state.zoom, zoom_delta);
        }

        // Escape: clear selection
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.state.selection.clear();
        }

        // Ctrl+A: select all slots on current page
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::A)) {
            let current_page =
                self.state
                    .hovered_slot
                    .map(|(p, _)| p)
                    .or(match &self.state.selection {
                        Selection::OnPage { page, .. } => Some(*page),
                        Selection::None => None,
                    });
            if let Some(page) = current_page {
                let slot_count = self
                    .state
                    .project_state
                    .layout
                    .get(page)
                    .map(|lp| lp.slots.len())
                    .unwrap_or(0);
                self.state.selection.select_all_on(page, slot_count);
            }
        }

        // Primary click: update selection based on hovered_slot from previous frame
        let clicked = ctx.input(|i| i.pointer.primary_clicked());
        if clicked {
            let modifiers = ctx.input(|i| i.modifiers);
            if let Some((page, slot)) = self.state.hovered_slot {
                if modifiers.shift {
                    self.state.selection.range_to(page, slot);
                } else if modifiers.ctrl || modifiers.command {
                    self.state.selection.toggle(page, slot);
                } else {
                    self.state.selection = Selection::single(page, slot);
                }
            } else {
                self.state.selection.clear();
            }
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
        self.handle_input(&ctx);
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
