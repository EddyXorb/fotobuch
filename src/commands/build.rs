//! `fotobuch build` command - Calculate layout and generate preview PDF

use anyhow::Result;
use crate::cache::preview;
use crate::output::typst;
use crate::solver::{run_solver, Request, RequestType};
use crate::state_manager::StateManager;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicUsize;

/// DPI warning for final build
#[derive(Debug)]
pub struct DpiWarning {
    /// Photo ID with low DPI
    pub photo_id: String,
    /// Actual DPI in the slot
    pub actual_dpi: f64,
    /// Page number where this occurs
    pub page: usize,
    /// Original dimensions in pixels
    pub original_px: (u32, u32),
    /// Slot dimensions in mm
    pub slot_mm: (f64, f64),
}

/// Configuration for build command
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// Build final PDF instead of preview (default: false)
    pub release: bool,
    /// Only build these pages (optional, default: all)
    pub pages: Option<Vec<usize>>,
}

/// Result of build command
#[derive(Debug)]
pub struct BuildResult {
    /// Path to generated PDF
    pub pdf_path: PathBuf,
    /// Pages that were rebuilt (1-based page numbers)
    pub pages_rebuilt: Vec<usize>,
    /// Pages with only swaps (no layout changes, 1-based)
    pub pages_swapped: Vec<usize>,
    /// Number of images processed in cache
    pub images_processed: usize,
    /// Total fitness cost
    pub total_cost: f64,
    /// DPI warnings (only for release builds)
    pub dpi_warnings: Vec<DpiWarning>,
    /// True if nothing needed to be done
    pub nothing_to_do: bool,
}

/// Calculate layout and generate preview or final PDF
///
/// # Steps
/// ## For first build (no layout in YAML):
/// 1. Preview cache: generate missing/stale preview images + watermark
/// 2. Book-Layout-Solver: distribute all photos from photos onto pages
/// 3. Page-Layout-Solver (GA): run_ga() for each page -> write layout[].slots
/// 4. Write fotobuch.yaml
/// 5. Compile Typst -> PDF
/// 6. Git commit: "post-build: N pages (cost: X)"
///
/// ## For incremental build (layout exists):
/// 1. Preview cache: check and regenerate changed images
/// 2. Compare with last git commit to find modified pages:
///    - Photos added/removed (length of photos changed)
///    - Photo swapped with different ratio
///    - area_weight changed in photos
/// 3. Page-Layout-Solver only for modified pages
/// 4. If nothing changed: "Nothing to do."
/// 5. Write fotobuch.yaml, compile Typst, git commit
///
/// ## For release build (--release):
/// 1. Check layout is clean (no uncommitted changes)
/// 2. Generate final cache: for each photo:
///    - Calculate target pixels from slot_mm and 300 DPI
///    - Always resample from original (no incremental)
///    - No watermark, high JPEG quality
/// 3. Compile fotobuch_final.typ -> final PDF
/// 4. Validate all images reach 300 DPI, collect warnings
/// 5. Git commit: "release: N pages, M photos"
///
/// # Arguments
/// * `project_root` - Path to the project directory
/// * `config` - Build configuration
///
/// # Returns
/// * `BuildResult` with PDF path, statistics, and warnings
pub fn build(project_root: &Path, config: &BuildConfig) -> Result<BuildResult> {
    let mgr = StateManager::open(project_root)?;

    // Handle release builds separately
    if config.release {
        if config.pages.is_some() {
            anyhow::bail!("--pages is not allowed with --release (must build entire book)");
        }
        // TODO: Implement release_build
        anyhow::bail!("Release build not yet implemented");
    }

    // First build vs incremental build
    if mgr.state.layout.is_empty() {
        first_build(mgr, project_root)
    } else {
        // TODO: Implement incremental_build
        anyhow::bail!("Incremental build not yet implemented");
    }
}

/// Performs the first build: generates layout for all photos and creates preview PDF.
fn first_build(mut mgr: StateManager, project_root: &Path) -> Result<BuildResult> {
    println!("First build: creating layout for all photos...");

    // 1. Generate preview cache for all photos
    let progress = AtomicUsize::new(0);
    let preview_cache_dir = mgr.preview_cache_dir();
    let cache_result = preview::ensure_previews(&mgr.state, &preview_cache_dir, &progress)?;

    println!(
        "Preview cache: {} created, {} skipped, {} total",
        cache_result.created, cache_result.skipped, cache_result.total
    );

    // 2. Run MultiPage solver to distribute photos across pages
    let request = Request {
        request_type: RequestType::MultiPage,
        groups: &mgr.state.photos,
        config: &mgr.state.config.book_layout_solver,
        ga_config: &mgr.state.config.page_layout_solver,
        book_config: &mgr.state.config.book,
    };

    let pages = run_solver(&request)?;
    let total_cost = calculate_total_cost(&pages);

    println!("Solver: generated {} pages", pages.len());

    // 3. Update state with layout
    mgr.state.layout = pages;

    // 4. Compile Typst template to PDF
    let pdf_path = typst::compile_preview(project_root, mgr.project_name())?;
    println!("PDF generated: {}", pdf_path.display());

    // 5. Save state and commit
    let pages_rebuilt: Vec<usize> = (1..=mgr.state.layout.len()).collect();
    let page_count = mgr.state.layout.len();
    
    mgr.finish(&format!(
        "build: {} pages (cost: {:.4})",
        page_count,
        total_cost
    ))?;

    Ok(BuildResult {
        pdf_path,
        pages_rebuilt,
        pages_swapped: vec![],
        images_processed: cache_result.created,
        total_cost,
        dpi_warnings: vec![],
        nothing_to_do: false,
    })
}

/// Calculates total cost from all pages.
/// TODO: Get this from solver result once available.
fn calculate_total_cost(pages: &[crate::dto_models::LayoutPage]) -> f64 {
    // Simple heuristic: sum of slot areas / page area
    // Better: get actual fitness scores from solver
    pages.len() as f64 * 0.85 // Placeholder
}
