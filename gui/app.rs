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
        let (task_tx, result_rx) =
            crate::background::spawn(project_root, project_name, cc.egui_ctx.clone());
        let _ = task_tx.send(BackgroundTask::RenderPages {
            pages: (0..num_pages).collect(),
            pixel_per_pt: state.base_pixel_per_pt,
        });

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
                } => {
                    let page_idx = full.page;
                    state::apply_rendered(&mut self.state, ctx, full, thumb);
                    self.state.timings.record_render(
                        page_idx,
                        rasterize_duration,
                        compile_duration,
                    );
                }
                BackgroundResult::CommandDone {
                    new_state,
                    dirty_pages,
                } => {
                    if let Some(new_state) = new_state {
                        let num_pages = new_state.layout.len();
                        self.state.project_state = *new_state;
                        self.state.derived =
                            crate::state::DerivedState::rebuild(&self.state.project_state);
                        state::resize_page_vecs(&mut self.state, num_pages);
                        // Rebuild prefetch queue after command
                        let loaded_or_inflight: HashSet<String> = self
                            .state
                            .photo_thumbs
                            .keys()
                            .chain(self.state.photo_thumb_in_flight.iter())
                            .cloned()
                            .collect();
                        self.state.photo_thumb_prefetch = self
                            .state
                            .derived
                            .photo_by_id
                            .keys()
                            .filter(|id| !loaded_or_inflight.contains(*id))
                            .cloned()
                            .collect();
                        for &p in &dirty_pages {
                            if let Some(d) = self.state.page_dirty.get_mut(p) {
                                *d = true;
                            }
                        }
                    } else {
                        for d in &mut self.state.page_dirty {
                            *d = false;
                        }
                    }
                }
                BackgroundResult::CommandFailed(e) => {
                    tracing::error!(%e, "command failed");
                    for d in &mut self.state.page_dirty {
                        *d = false;
                    }
                }
                BackgroundResult::Error(e) => {
                    tracing::error!(%e, "render error");
                }
                BackgroundResult::PhotoThumbnailReady {
                    id,
                    width,
                    height,
                    pixels,
                } => {
                    if !self.state.derived.photo_by_id.contains_key(&id) {
                        self.state.photo_thumb_in_flight.remove(&id);
                        continue;
                    }
                    let img = egui::ColorImage::from_rgba_unmultiplied(
                        [width as _, height as _],
                        &pixels,
                    );
                    let tex =
                        ctx.load_texture(format!("thumb_{id}"), img, egui::TextureOptions::LINEAR);
                    self.state.photo_thumbs.insert(id.clone(), tex);
                    self.state.photo_thumb_in_flight.remove(&id);
                }
            }
        }
    }

    fn dispatch_commands(&mut self, cmds: HashSet<PendingCommand>) {
        let ppt = self.state.base_pixel_per_pt;
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
                        if let Some(d) = self.state.page_dirty.get_mut(p) {
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
                    for d in &mut self.state.page_dirty {
                        *d = true;
                    }
                    BackgroundTask::ConfigSet {
                        key,
                        value,
                        pixel_per_pt: ppt,
                    }
                }
            };
            let _ = self.task_tx.send(task);
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
        widgets::statusbar::draw(ui, &mut self.state);

        // Side panels must come before the central panel (egui ordering requirement).
        widgets::photo_pool::draw(ui, &mut self.state, &mut cmds);
        widgets::page_nav::draw(ui, &mut self.state, &mut cmds);

        let t = Instant::now();
        widgets::central_panel::draw(ui, &mut self.state);
        self.state.timings.show_pages = t.elapsed();

        if self.state.config_panel.open {
            widgets::config_window::show(&ctx, &mut self.state, &mut cmds);
        }

        // Reset per-frame flag after all widgets have drawn.
        self.state.highlight_duplicates = false;

        // Flush pending thumbnail loads collected during widget drawing.
        if let Some(task) = widgets::photo_pool::flush_thumb_loads(&mut self.state) {
            let _ = self.task_tx.send(task);
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
