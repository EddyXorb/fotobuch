mod input_handler;
mod pending;
mod widgets;

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use crossbeam::channel::{Receiver, Sender};
use fotobuch::state_manager::StateManager;

use crate::state::{self, GuiState};
use crate::task::{BackgroundResult, BackgroundTask};
use fotobuch::dto_models::ProjectState;
use fotobuch::output::typst::RenderedPage;
use std::time::Duration;

use pending::PendingCommand;

pub struct FotobuchApp {
    state: GuiState,
    task_tx: Sender<BackgroundTask>,
    result_rx: Receiver<BackgroundResult>,
}

impl FotobuchApp {
    pub fn new(cc: &eframe::CreationContext, project_root: PathBuf) -> anyhow::Result<Self> {
        let mgr = StateManager::open(&project_root)?;
        let project_name = mgr.project_name().to_owned();
        let num_pages = mgr.state.layout.len();
        let project_state = mgr.state.clone();
        drop(mgr);

        let state = GuiState::new(project_state);
        cc.egui_ctx
            .global_style_mut(|s| s.interaction.tooltip_delay = 0.05);
        let (task_tx, result_rx) =
            crate::background::spawn(project_root, project_name, cc.egui_ctx.clone());
        if task_tx
            .send(BackgroundTask::RenderPages {
                pages: (0..num_pages).collect(),
                pixel_per_pt: state.viewport.pixel_per_pt,
            })
            .is_err()
        {
            tracing::error!("background worker closed before initial render was sent");
        }

        Ok(Self {
            state,
            task_tx,
            result_rx,
        })
    }

    fn drain_results(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.result_rx.try_recv() {
            match msg {
                BackgroundResult::PageRendered {
                    page: full,
                    thumb,
                    rasterize_duration,
                    compile_duration,
                } => self.handle_page_rendered(
                    ctx,
                    full,
                    thumb,
                    rasterize_duration,
                    compile_duration,
                ),
                BackgroundResult::CommandDone {
                    new_state,
                    dirty_pages,
                } => {
                    self.handle_command_done(new_state, dirty_pages);
                }
                BackgroundResult::CommandFailed(e) => self.handle_command_failed(e),
                BackgroundResult::Error(e) => tracing::error!(%e, "render error"),
                BackgroundResult::TotalPageCount(n) => state::resize_page_vecs(&mut self.state, n),
                BackgroundResult::PhotoThumbnailReady {
                    id,
                    width,
                    height,
                    pixels,
                } => {
                    if !self.handle_photo_thumbnail_ready(ctx, id, width, height, pixels) {
                        continue;
                    }
                }
            }
        }
    }

    fn handle_page_rendered(
        &mut self,
        ctx: &egui::Context,
        full: RenderedPage,
        thumb: RenderedPage,
        rasterize_duration: Duration,
        compile_duration: Duration,
    ) {
        let page_idx = full.page;
        state::apply_rendered(&mut self.state, ctx, full, thumb);
        self.state
            .timings
            .record_render(page_idx, rasterize_duration, compile_duration);
    }

    fn handle_command_done(
        &mut self,
        new_state: Option<Box<ProjectState>>,
        dirty_pages: Vec<usize>,
    ) {
        if let Some(new_state) = new_state {
            let num_pages = new_state.layout.len();
            self.state.project_state = *new_state;
            self.state.derived = crate::state::DerivedState::rebuild(&self.state.project_state);
            state::resize_page_vecs(&mut self.state, num_pages);
            let loaded_or_inflight: HashSet<String> = self
                .state
                .thumb
                .thumbs
                .keys()
                .chain(self.state.thumb.in_flight.iter())
                .cloned()
                .collect();
            self.state.thumb.prefetch = self
                .state
                .derived
                .photo_by_id
                .keys()
                .filter(|id| !loaded_or_inflight.contains(*id))
                .cloned()
                .collect();
            for &p in &dirty_pages {
                if let Some(d) = self.state.cache.dirty.get_mut(p) {
                    *d = true;
                }
            }
        } else {
            for d in &mut self.state.cache.dirty {
                *d = false;
            }
        }
    }

    fn handle_command_failed(&mut self, e: String) {
        tracing::error!(%e, "command failed");
        for d in &mut self.state.cache.dirty {
            *d = false;
        }
    }

    /// Returns `false` if the loop should `continue` (photo not in project).
    fn handle_photo_thumbnail_ready(
        &mut self,
        ctx: &egui::Context,
        id: String,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> bool {
        if !self.state.derived.photo_by_id.contains_key(&id) {
            self.state.thumb.in_flight.remove(&id);
            return false;
        }
        let img = egui::ColorImage::from_rgba_unmultiplied([width as _, height as _], &pixels);
        let tex = ctx.load_texture(format!("thumb_{id}"), img, egui::TextureOptions::LINEAR);
        self.state.thumb.thumbs.insert(id.clone(), tex);
        self.state.thumb.in_flight.remove(&id);
        true
    }

    fn dispatch_commands(&mut self, cmds: HashSet<PendingCommand>) {
        let ppt = self.state.viewport.pixel_per_pt;
        for cmd in cmds {
            let task = match cmd {
                PendingCommand::Swap {
                    src_page,
                    src_slot,
                    dst_page,
                    dst_slot,
                } => BackgroundTask::SwapSlots {
                    src_page,
                    src_slot,
                    dst_page,
                    dst_slot,
                    pixel_per_pt: ppt,
                },
                PendingCommand::Move {
                    src_page,
                    src_slots,
                    dst_page,
                } => BackgroundTask::MoveSlot {
                    src_page,
                    src_slots,
                    dst_page,
                    pixel_per_pt: ppt,
                },
                PendingCommand::Undo => BackgroundTask::Undo { pixel_per_pt: ppt },
                PendingCommand::Redo => BackgroundTask::Redo { pixel_per_pt: ppt },
                PendingCommand::Place {
                    photo_ids,
                    dst_page,
                } => BackgroundTask::PlacePhotos {
                    photo_ids,
                    dst_page,
                    pixel_per_pt: ppt,
                },
                PendingCommand::PageSwap { left, right } => {
                    for &p in &[left, right] {
                        if let Some(d) = self.state.cache.dirty.get_mut(p) {
                            *d = true;
                        }
                    }
                    BackgroundTask::PageSwap {
                        left,
                        right,
                        pixel_per_pt: ppt,
                    }
                }
                PendingCommand::ConfigSet { key, value } => {
                    for d in &mut self.state.cache.dirty {
                        *d = true;
                    }
                    BackgroundTask::ConfigSet {
                        key,
                        value,
                        pixel_per_pt: ppt,
                    }
                }
            };
            if self.task_tx.send(task).is_err() {
                tracing::error!("background worker closed; command dropped");
            }
        }
    }
}

impl eframe::App for FotobuchApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let t_frame = Instant::now();
        self.state.timings.frame_cnt += 1;
        let ctx = ui.ctx().clone();

        let t = Instant::now();
        self.drain_results(&ctx);
        self.state.timings.drain_results = t.elapsed();

        let mut cmds = widgets::toolbar::draw(ui, &mut self.state);
        widgets::statusbar::draw(ui, &self.state);

        // Clear per-frame hover state before widgets re-populate it.
        self.state.hovered = None;

        // Side panels must come before the central panel (egui ordering requirement).
        widgets::photo_pool::draw(ui, &mut self.state);
        widgets::page_nav::draw(ui, &mut self.state, &mut cmds);

        let t = Instant::now();
        widgets::central_panel::draw(ui, &mut self.state);
        self.state.timings.show_pages = t.elapsed();

        if self.state.config_panel.open {
            widgets::config_window::show(&ctx, &mut self.state, &mut cmds);
        }

        // Flush pending thumbnail loads collected during widget drawing.
        if let Some(task) = widgets::photo_pool::flush_thumb_loads(&mut self.state) {
            if self.task_tx.send(task).is_err() {
                tracing::error!("background worker closed; thumbnail load dropped");
            }
        }

        // Input handling runs after central panel so that hovered_slot reflects the current
        // frame — prevents toolbar clicks from accidentally triggering a drag.
        let t = Instant::now();
        cmds.extend(input_handler::handle(&mut self.state, &ctx));
        self.dispatch_commands(cmds);
        self.state.timings.input_handlers = t.elapsed();

        self.state.timings.ui_frame = t_frame.elapsed();

        if self.state.timings.show {
            widgets::timings_panel::draw(&self.state.timings, &ctx);
        }
    }
}
