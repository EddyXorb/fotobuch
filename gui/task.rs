use std::path::PathBuf;
use std::time::Duration;

use fotobuch::commands::PlaceDst;
use fotobuch::commands::history::HistoryEntry;
use fotobuch::commands::project::{NewConfig, ProjectInfo};
use fotobuch::dto_models::ProjectState;
use fotobuch::output::typst::RenderedPage;

pub enum BackgroundTask {
    RenderPages {
        pages: Vec<usize>,
        pixel_per_pt: f32,
    },
    SetPixelPerPt(f32),
    Swap {
        src_page: usize,
        src_slot: usize,
        dst_page: usize,
        dst_slot: usize,
    },
    Move {
        src_page: usize,
        src_slots: Vec<usize>,
        dst_page: usize,
    },
    Undo,
    Redo,
    /// Nav-Drag: ganze Seiten tauschen.
    PageSwap {
        left: usize,
        right: usize,
    },
    /// Foto-Thumbnails laden (Pool-Panel).
    LoadPhotoThumbnails {
        items: Vec<(String, PathBuf)>,
    },
    /// Fotos platzieren — konkrete Zielseite oder auto-distribute.
    Place {
        photo_ids: Vec<String>,
        dst: PlaceDst,
    },
    /// `config set key value` im Background.
    ConfigSet {
        key: String,
        value: String,
    },
    MoveToNewPage {
        src_page: usize,
        src_slots: Vec<usize>,
        at_position: usize,
    },
    /// Move slots onto a Manual page, placing the dragged photo's upper-left at
    /// `(x_mm, y_mm)` while keeping each moved slot's size.
    MoveToManual {
        src_page: usize,
        src_slots: Vec<usize>,
        dst_page: usize,
        x_mm: f64,
        y_mm: f64,
    },
    SwapRange {
        src_page: usize,
        src_slots: Vec<usize>,
        dst_page: usize,
        dst_slots: Vec<usize>,
    },
    Unplace {
        page: usize,
        slots: Vec<usize>,
    },
    DeletePages {
        pages: Vec<usize>,
    },
    MovePage {
        src_page: usize,
        at_position: usize,
    },
    RebuildPages {
        pages: Vec<usize>,
    },
    RebuildAll,
    ReleaseBuild,
    AddPhotos {
        paths: Vec<PathBuf>,
        recursive: bool,
        weight: f64,
        source_filter: String,
    },
    RemovePhotos {
        photo_ids: Vec<String>,
    },
    SetPageMode {
        page: usize,
        mode: fotobuch::dto_models::PageMode,
    },
    PagePos {
        page: usize,
        slot: usize,
        mode: PagePosMode,
        scale: Option<f64>,
    },
    SetWeight {
        page: usize,
        slots: Vec<usize>,
        weight: f64,
    },
    /// Create a new project in the vault and switch to it.
    ProjectNew {
        config: NewConfig,
    },
    /// Switch to a different project branch.
    ProjectSwitch {
        name: String,
    },
    /// List all projects in the vault (returns ProjectList result).
    ListProjects,
    /// Switch the active vault to a new directory.
    SwitchVault(PathBuf),
    /// Load the last `count` history entries for the current branch.
    LoadHistory {
        count: usize,
    },
}

#[derive(Debug)]
pub enum PagePosMode {
    Relative { dx_mm: f64, dy_mm: f64 },
    Absolute { x_mm: f64, y_mm: f64 },
}

#[derive(Debug)]
pub enum BackgroundResult {
    PageRendered {
        page: RenderedPage,
        /// Downsample (längste Kante ~120 px) für das Nav-Panel.
        thumb: RenderedPage,
        /// Time spent rasterising this single page.
        rasterize_duration: Duration,
        /// Time spent on `compile_document` for the task this page belongs to.
        /// All pages from the same task share the same value.
        compile_duration: Duration,
    },
    /// A command completed successfully.
    CommandDone {
        /// Updated project state, or `None` if the state did not change.
        new_state: Option<Box<ProjectState>>,
        /// Page indices that changed and need re-rendering.
        dirty_pages: Vec<usize>,
    },
    /// A command failed (user-visible error, not a render error).
    CommandFailed(String),
    Error(String),
    /// Result of `ListProjects`.
    ProjectList {
        projects: Vec<ProjectInfo>,
    },
    /// The vault has been switched; contains updated path and project list.
    VaultSwitched {
        vault_path: std::path::PathBuf,
        projects: Vec<ProjectInfo>,
    },
    /// Result of `LoadHistory`.
    HistoryLoaded {
        entries: Vec<HistoryEntry>,
    },
    /// Total number of pages in the compiled Typst document (may exceed layout.len()
    /// when appendix or other extra pages are active).
    TotalPageCount(usize),
    PhotoThumbnailReady {
        id: String,
        width: u32,
        height: u32,
        /// Straight-alpha RGBA.
        pixels: Vec<u8>,
    },
}
