//! `fotobuch build` command - Calculate layout and generate preview PDF

use anyhow::Result;
use crate::cache::{final_cache, preview};
use crate::dto_models::{PhotoFile, PhotoGroup};
use crate::output::typst;
use crate::solver::{run_solver, Request, RequestType};
use crate::state_manager::StateManager;
use std::collections::HashMap;
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
        return release_build(mgr, project_root);
    }

    // First build vs incremental build
    if mgr.state.layout.is_empty() {
        first_build(mgr, project_root)
    } else {
        incremental_build(mgr, project_root, config.pages.as_deref())
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

/// Performs incremental build: updates only modified pages.
fn incremental_build(
    mut mgr: StateManager,
    project_root: &Path,
    page_filter: Option<&[usize]>,
) -> Result<BuildResult> {
    println!("Incremental build: checking for changes...");

    // 1. Generate/update preview cache
    let progress = AtomicUsize::new(0);
    let preview_cache_dir = mgr.preview_cache_dir();
    let cache_result = preview::ensure_previews(&mgr.state, &preview_cache_dir, &progress)?;

    if cache_result.created > 0 {
        println!(
            "Preview cache: {} created, {} skipped",
            cache_result.created, cache_result.skipped
        );
    }

    // 2. Detect which pages need rebuilding
    let pages_needing_rebuild = mgr.modified_pages();

    // 3. Apply page filter if specified
    let pages_needing_rebuild = apply_page_filter(pages_needing_rebuild, page_filter);

    if pages_needing_rebuild.is_empty() {
        println!("No changes detected. Nothing to do.");
        return Ok(BuildResult {
            pdf_path: project_root.join(format!("{}.pdf", mgr.project_name())),
            pages_rebuilt: vec![],
            pages_swapped: vec![],
            images_processed: cache_result.created,
            total_cost: 0.0,
            dpi_warnings: vec![],
            nothing_to_do: true,
        });
    }

    println!("Rebuilding {} page(s): {:?}", pages_needing_rebuild.len(), pages_needing_rebuild);

    // 4. Build photo index for fast lookup
    let photo_index = build_photo_index(&mgr.state.photos);

    // 5. Rebuild each modified page
    for &page_num in &pages_needing_rebuild {
        rebuild_single_page(&mut mgr.state, page_num, &photo_index)?;
    }

    // 6. Compile Typst template to PDF
    let pdf_path = typst::compile_preview(project_root, mgr.project_name())?;
    println!("PDF updated: {}", pdf_path.display());

    // 7. Save state and commit
    let total_cost = calculate_total_cost(&mgr.state.layout);
    mgr.finish(&format!(
        "build: {} page(s) rebuilt",
        pages_needing_rebuild.len()
    ))?;

    Ok(BuildResult {
        pdf_path,
        pages_rebuilt: pages_needing_rebuild,
        pages_swapped: vec![],
        images_processed: cache_result.created,
        total_cost,
        dpi_warnings: vec![],
        nothing_to_do: false,
    })
}

/// Applies page filter to the list of pages needing rebuild.
/// If filter is None, returns all pages. Otherwise returns only pages in the filter.
fn apply_page_filter(mut pages: Vec<usize>, filter: Option<&[usize]>) -> Vec<usize> {
    if let Some(filter_pages) = filter {
        pages.retain(|p| filter_pages.contains(p));
    }
    pages
}

/// Builds a photo index for fast lookup: photo_id -> (PhotoFile, group_name).
fn build_photo_index(photos: &[PhotoGroup]) -> HashMap<String, (PhotoFile, String)> {
    photos
        .iter()
        .flat_map(|group| {
            group.files.iter().map(move |file| {
                (file.id.clone(), (file.clone(), group.group.clone()))
            })
        })
        .collect()
}

/// Rebuilds a single page using the SinglePage solver.
/// Page number is 1-based, converted to 0-based index internally.
fn rebuild_single_page(
    state: &mut crate::dto_models::ProjectState,
    page_num: usize,
    photo_index: &HashMap<String, (PhotoFile, String)>,
) -> Result<()> {
    let page_idx = page_num - 1;

    if page_idx >= state.layout.len() {
        anyhow::bail!("Page {} does not exist (layout has {} pages)", page_num, state.layout.len());
    }

    let page = &state.layout[page_idx];

    // Build PhotoGroup from the page's photo IDs
    let files: Vec<PhotoFile> = page
        .photos
        .iter()
        .filter_map(|id| photo_index.get(id).map(|(file, _)| file.clone()))
        .collect();

    if files.is_empty() {
        anyhow::bail!("Page {} has no valid photos", page_num);
    }

    let group = PhotoGroup {
        group: format!("page_{}", page_num),
        sort_key: String::new(),
        files,
    };

    // Run SinglePage solver
    let request = Request {
        request_type: RequestType::SinglePage,
        groups: &[group],
        config: &state.config.book_layout_solver,
        ga_config: &state.config.page_layout_solver,
        book_config: &state.config.book,
    };

    let result = run_solver(&request)?;

    if result.is_empty() {
        anyhow::bail!("Solver returned no result for page {}", page_num);
    }

    // Update only the slots, keep photos list unchanged
    state.layout[page_idx].slots = result[0].slots.clone();

    Ok(())
}

/// Performs release build: generates final high-quality PDF at 300 DPI.
///
/// # Requirements
/// - Layout must be clean (no uncommitted changes)
/// - All photos must be available
///
/// # Steps
/// 1. Verify layout is clean
/// 2. Generate final cache (300 DPI) and collect DPI warnings
/// 3. Compile final.typ -> final.pdf
/// 4. Save and commit
fn release_build(mgr: StateManager, project_root: &Path) -> Result<BuildResult> {
    println!("Release build: generating final PDF at 300 DPI...");

    // 1. Check that layout is clean (no changes since last build)
    if mgr.has_changes_since_last_build() {
        anyhow::bail!(
            "Layout has changes since last build. Run `fotobuch build` first to commit all changes."
        );
    }

    if mgr.state.layout.is_empty() {
        anyhow::bail!("No layout found. Run `fotobuch build` first to generate layout.");
    }

    // 2. Generate final cache at 300 DPI
    let progress = AtomicUsize::new(0);
    let final_cache_dir = mgr.final_cache_dir();
    let final_result = final_cache::build_final_cache(&mgr.state, &final_cache_dir, &progress)?;

    println!(
        "Final cache: {} images generated, {} DPI warnings",
        final_result.created,
        final_result.dpi_warnings.len()
    );

    // Print DPI warnings
    if !final_result.dpi_warnings.is_empty() {
        println!("\nWARNING: Some photos will be displayed below 300 DPI:");
        for warning in &final_result.dpi_warnings {
            println!(
                "  Page {}: {} - {:.1} DPI ({}x{} px in {:.1}x{:.1} mm slot)",
                warning.page,
                warning.photo_id,
                warning.actual_dpi,
                warning.original_px.0,
                warning.original_px.1,
                warning.slot_mm.0,
                warning.slot_mm.1
            );
        }
        println!();
    }

    // 3. Compile final.typ -> final.pdf
    let pdf_path = typst::compile_final(project_root, mgr.project_name())?;
    println!("Final PDF generated: {}", pdf_path.display());

    // 4. Save state and commit
    let page_count = mgr.state.layout.len();
    let total_photos: usize = mgr.state.layout.iter().map(|p| p.photos.len()).sum();

    mgr.finish_always(&format!(
        "release: {} pages, {} photos",
        page_count, total_photos
    ))?;

    Ok(BuildResult {
        pdf_path,
        pages_rebuilt: vec![], // Release doesn't rebuild layout
        pages_swapped: vec![],
        images_processed: final_result.created,
        total_cost: 0.0, // Not relevant for release
        dpi_warnings: final_result.dpi_warnings,
        nothing_to_do: false,
    })
}
