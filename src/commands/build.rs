//! `fotobuch build` command - Calculate layout and generate preview PDF

use anyhow::Result;
use std::path::{Path, PathBuf};

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
    /// Number of pages built
    pub pages_built: usize,
    /// Number of images cached/processed
    pub images_processed: usize,
    /// Total fitness cost
    pub total_cost: f64,
    /// DPI warnings (only for release builds)
    pub dpi_warnings: Vec<DpiWarning>,
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
    // TODO: Implement build command
    // - Load fotobuch.yaml
    // - Check if first build or incremental
    // - If release: verify clean state
    // - Generate appropriate cache (preview or final)
    // - Run solvers as needed
    // - Compile Typst
    // - Git commits (pre/post)
    // - For release: validate DPI

    let _ = (project_root, config); // Silence unused warnings

    Ok(BuildResult {
        pdf_path: PathBuf::from("fotobuch_preview.pdf"),
        pages_built: 0,
        images_processed: 0,
        total_cost: 0.0,
        dpi_warnings: Vec::new(),
    })
}
