pub(crate) mod help;
mod input_handler;
pub(super) mod rebuild;
mod widgets;

use std::path::PathBuf;
use std::time::Instant;

use crossbeam::channel::{Receiver, Sender};
use fotobuch::state_manager::StateManager;

use crate::state::{self, GuiState};
use crate::task::{BackgroundResult, BackgroundTask};
use fotobuch::commands::PlaceDst;
use fotobuch::dto_models::ProjectState;
use fotobuch::output::typst::RenderedPage;
use std::time::Duration;

pub struct FotobuchApp {
    state: GuiState,
    task_tx: Sender<BackgroundTask>,
    result_rx: Receiver<BackgroundResult>,
}

impl FotobuchApp {
    pub fn new(
        cc: &eframe::CreationContext,
        vault_path: PathBuf,
        show_welcome: bool,
    ) -> anyhow::Result<Self> {
        let (project_state, project_name, projects) = if show_welcome {
            (ProjectState::default(), String::new(), Vec::new())
        } else {
            let mgr = StateManager::open(&vault_path)?;
            let project_name = mgr.project_name().to_owned();
            let num_pages = mgr.state.layout.len();
            let project_state = mgr.state.clone();
            drop(mgr);

            let projects = fotobuch::commands::project::project_list(&vault_path)
                .map(|o| o.result)
                .unwrap_or_default();

            let _ = num_pages; // used below
            (project_state, project_name, projects)
        };

        let num_pages = project_state.layout.len();
        let state = GuiState::new(project_state, vault_path.clone(), projects, show_welcome);
        install_fallback_font(&cc.egui_ctx);
        cc.egui_ctx
            .global_style_mut(|s| s.interaction.tooltip_delay = 0.0);

        let (task_tx, result_rx) =
            crate::background::spawn(vault_path, project_name, cc.egui_ctx.clone());

        if !show_welcome {
            render_initial_pages(num_pages, &state, &task_tx);
            create_initial_photo_thumbs(&state, &task_tx);
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
                BackgroundResult::ProjectList { projects } => {
                    self.state.data.projects = projects;
                }
                BackgroundResult::VaultSwitched {
                    vault_path,
                    projects,
                } => {
                    self.state.data.vault_path = vault_path.clone();
                    if let Ok(mut settings) = fotobuch::app_settings::AppSettings::load() {
                        settings.add_recent_vault(&vault_path);
                        let _ = settings.save();
                    }
                    if projects.is_empty() {
                        self.state.data.projects = projects;
                        self.state.interaction.show_welcome = true;
                    } else {
                        let first_name = projects[0].name.clone();
                        self.state.data.projects = projects;
                        let _ = self
                            .task_tx
                            .send(BackgroundTask::ProjectSwitch { name: first_name });
                    }
                }
                BackgroundResult::HistoryLoaded { entries } => {
                    self.state.data.history = entries;
                }
                BackgroundResult::ReleaseDone { pdf_path } => {
                    let _ = open::that_detached(&pdf_path);
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
        self.state.interaction.timings.record_render(
            page_idx,
            rasterize_duration,
            compile_duration,
        );
    }

    fn handle_command_done(
        &mut self,
        new_state: Option<Box<ProjectState>>,
        dirty_pages: Vec<usize>,
    ) {
        if let Some(new_state) = new_state {
            let num_pages = new_state.layout.len();
            self.state.data.project = *new_state;
            self.state.data.derived = crate::state::DerivedState::rebuild(&self.state.data.project);
            state::resize_page_vecs(&mut self.state, num_pages);
            let items: Vec<(std::path::PathBuf, String)> = self
                .state
                .data
                .project
                .photos
                .iter()
                .flat_map(|g| g.files.iter())
                .filter(|f| !self.state.data.thumbs.contains_key(&f.id))
                .map(|f| (PathBuf::from(&f.source), f.id.clone()))
                .collect();
            if !items.is_empty() {
                let items = items.into_iter().map(|(p, id)| (id, p)).collect();
                if self
                    .task_tx
                    .send(BackgroundTask::LoadPhotoThumbnails { items })
                    .is_err()
                {
                    tracing::error!("background worker closed; thumb load dropped");
                }
            }
            unset_dirty_pages(&mut self.state.data.pages.dirty);
            for &p in &dirty_pages {
                if let Some(d) = self.state.data.pages.dirty.get_mut(p) {
                    *d = true;
                }
            }
            self.state.interaction.selections.slots.clear();
            self.state.interaction.selections.nav_pages.clear();
            // Hide welcome modal after first project creation
            self.state.interaction.show_welcome = false;
            // Invalidate history cache on every state change
            self.state.data.history.clear();
            // Refresh project list so the switcher dropdown stays current
            let _ = self.task_tx.send(BackgroundTask::ListProjects);
        } else {
            unset_dirty_pages(&mut self.state.data.pages.dirty);
        }
    }

    fn handle_command_failed(&mut self, e: String) {
        tracing::error!(%e, "command failed");
        self.state.interaction.toasts.push(e);
        unset_dirty_pages(&mut self.state.data.pages.dirty);
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
        if !self.state.data.derived.photo_by_id.contains_key(&id) {
            return false;
        }
        let img = egui::ColorImage::from_rgba_unmultiplied([width as _, height as _], &pixels);
        let tex = ctx.load_texture(format!("thumb_{id}"), img, egui::TextureOptions::LINEAR);
        self.state.data.thumbs.insert(id, tex);
        true
    }

    fn dispatch_commands(&mut self, cmds: Vec<BackgroundTask>) {
        if cmds.is_empty() {
            return;
        }
        let ppt = self.state.interaction.viewport.pixel_per_pt;
        if self
            .task_tx
            .send(BackgroundTask::SetPixelPerPt(ppt))
            .is_err()
        {
            tracing::error!("background worker closed; command dropped");
            return;
        }
        for task in cmds {
            mark_dirty(&mut self.state.data.pages.dirty, &task);
            if self.task_tx.send(task).is_err() {
                tracing::error!("background worker closed; command dropped");
            }
        }
    }
}

fn create_initial_photo_thumbs(state: &GuiState, task_tx: &Sender<BackgroundTask>) {
    let thumb_items: Vec<(String, PathBuf)> = state
        .data
        .project
        .photos
        .iter()
        .flat_map(|g| {
            g.files
                .iter()
                .map(|f| (f.id.clone(), PathBuf::from(&f.source)))
        })
        .collect();
    if !thumb_items.is_empty()
        && task_tx
            .send(BackgroundTask::LoadPhotoThumbnails { items: thumb_items })
            .is_err()
    {
        tracing::error!("background worker closed before initial thumb load was sent");
    }
}

fn render_initial_pages(num_pages: usize, state: &GuiState, task_tx: &Sender<BackgroundTask>) {
    if task_tx
        .send(BackgroundTask::RenderPages {
            pages: (0..num_pages).collect(),
            pixel_per_pt: state.interaction.viewport.pixel_per_pt,
        })
        .is_err()
    {
        tracing::error!("background worker closed before initial render was sent");
    }
}

fn unset_dirty_pages(dirty: &mut [bool]) {
    dirty.fill(false);
}

fn install_fallback_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "DejaVuSans".to_owned(),
        egui::FontData::from_static(include_bytes!("assets/DejaVuSans.ttf")).into(),
    );
    for family_fonts in fonts.families.values_mut() {
        family_fonts.push("DejaVuSans".to_owned());
    }
    ctx.set_fonts(fonts);
}

fn mark_dirty(dirty: &mut [bool], task: &BackgroundTask) {
    match task {
        BackgroundTask::Swap {
            src_page, dst_page, ..
        }
        | BackgroundTask::Move {
            src_page, dst_page, ..
        }
        | BackgroundTask::PageSwap {
            left: src_page,
            right: dst_page,
            ..
        }
        | BackgroundTask::MoveToManual {
            src_page, dst_page, ..
        } => {
            for &p in &[*src_page, *dst_page] {
                if let Some(d) = dirty.get_mut(p) {
                    *d = true;
                }
            }
        }
        BackgroundTask::Place {
            dst: PlaceDst::Page(p),
            ..
        } => {
            if let Some(d) = dirty.get_mut(*p) {
                *d = true;
            }
        }
        BackgroundTask::Unplace { page, .. } => {
            if let Some(d) = dirty.get_mut(*page) {
                *d = true;
            }
        }
        BackgroundTask::RebuildPages { pages, .. } => {
            for &p in pages {
                if let Some(d) = dirty.get_mut(p) {
                    *d = true;
                }
            }
        }
        BackgroundTask::SetWeight { page, .. } | BackgroundTask::SetPageMode { page, .. } => {
            if let Some(d) = dirty.get_mut(*page) {
                *d = true;
            }
        }
        BackgroundTask::Undo
        | BackgroundTask::Redo
        | BackgroundTask::ConfigSet { .. }
        | BackgroundTask::Place { .. }
        | BackgroundTask::MoveToNewPage { .. }
        | BackgroundTask::MovePage { .. }
        | BackgroundTask::SwapRange { .. }
        | BackgroundTask::DeletePages { .. }
        | BackgroundTask::RebuildAll
        | BackgroundTask::ReleaseBuild
        | BackgroundTask::AddPhotos { .. }
        | BackgroundTask::RemovePhotos { .. }
        | BackgroundTask::PagePos { .. }
        | BackgroundTask::ProjectNew { .. }
        | BackgroundTask::ProjectSwitch { .. } => {
            dirty.fill(true);
        }
        BackgroundTask::RenderPages { .. }
        | BackgroundTask::SetPixelPerPt(..)
        | BackgroundTask::LoadPhotoThumbnails { .. }
        | BackgroundTask::ListProjects
        | BackgroundTask::SwitchVault(..)
        | BackgroundTask::LoadHistory { .. } => {}
    }
}

impl eframe::App for FotobuchApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let t_frame = Instant::now();
        self.state.interaction.timings.frame_cnt += 1;
        let ctx = ui.ctx().clone();

        let t = Instant::now();
        self.drain_results(&ctx);
        self.state.interaction.timings.drain_results = t.elapsed();

        let t = Instant::now();
        let mut cmds =
            widgets::draw_widgets(ui, &ctx, &self.state.data, &mut self.state.interaction);
        self.state.interaction.timings.show_panels = t.elapsed();

        let t = Instant::now();
        cmds.extend(input_handler::handle(
            &self.state.data,
            &mut self.state.interaction,
            &ctx,
        ));
        self.dispatch_commands(cmds);

        self.state.interaction.timings.input_handlers = t.elapsed();
        self.state.interaction.timings.ui_frame = t_frame.elapsed();
        if self.state.interaction.timings.show {
            widgets::timings_panel::draw(&self.state.interaction.timings, &ctx);
        }
    }
}
