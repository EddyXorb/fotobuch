//! `fotobuch build` and `fotobuch rebuild` commands
mod core;
pub(super) mod cover;
mod helpers;
pub mod plan;

pub use core::multipage_build::{MultiPageParams, multipage_build};
pub use core::rebuild_single_page::rebuild_single_page;
pub use helpers::{
    CommitMode, PdfTarget, RenderContext, build_photo_index, collect_photos_as_groups, render_pdf,
};
pub use plan::{BuildPlan, RebuildScope};

use crate::state_manager::StateManager;
use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// DPI warning for final build
#[derive(Debug)]
pub struct DpiWarning {
    /// Photo ID with low DPI
    pub photo_id: String,
    /// Actual DPI in the slot
    pub actual_dpi: f64,
    /// 0-based page index (layout array position) where this occurs
    pub page: usize,
    /// Original dimensions in pixels
    pub original_px: (u32, u32),
    /// Slot dimensions in mm
    pub slot_mm: (f64, f64),
}

/// Options controlling the build pipeline's side effects.
#[derive(Debug, Clone, Copy)]
pub struct BuildOptions {
    pub skip_pdf: bool,
    pub skip_cache_update: bool,
}

/// Configuration for the `build` command.
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// Build final PDF instead of preview (default: false)
    pub release: bool,
    /// Force release even if layout has uncommitted changes (default: false)
    pub force: bool,
    /// Only process these pages (0-based indices, optional, default: all)
    pub pages: Option<Vec<usize>>,
    /// Skip PDF generation (Typst compilation). Only produce YAML layout.
    pub skip_pdf: bool,
    /// Skip preview cache update, only for preview builds. Use with caution.
    pub skip_cache_update: bool,
}

/// Result of build command
#[derive(Debug)]
pub struct BuildResult {
    /// Path to generated PDF
    pub pdf_path: PathBuf,
    /// Pages that were rebuilt (0-based array indices into layout[])
    pub pages_rebuilt: Vec<usize>,
    /// Pages with only swaps (no layout changes, 0-based indices)
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

/// Calculate layout and generate preview or final PDF.
pub fn build(
    project_root: &Path,
    config: &BuildConfig,
) -> Result<super::CommandOutput<BuildResult>> {
    let mgr = StateManager::open(project_root)?;
    let opts = BuildOptions {
        skip_pdf: config.skip_pdf || !mgr.state.config.preview.write_pdf,
        skip_cache_update: config.skip_cache_update,
    };
    let plan = BuildPlan::from_build_config(&mgr, config)?;
    plan.run(mgr, project_root, opts)
}

/// Force re-optimization of pages or page ranges.
pub fn rebuild(
    project_root: &Path,
    scope: RebuildScope,
    opts: BuildOptions,
) -> Result<super::CommandOutput<BuildResult>> {
    let mgr = StateManager::open(project_root)?;
    let opts = BuildOptions {
        skip_pdf: opts.skip_pdf || !mgr.state.config.preview.write_pdf,
        ..opts
    };
    let plan = BuildPlan::from_rebuild_scope(&mgr, scope)?;
    plan.run(mgr, project_root, opts)
}

/// Output build result summary (pages rebuilt, PDF path, DPI warnings).
pub fn print_build_result(result: &BuildResult) {
    if !result.pages_rebuilt.is_empty() {
        info!(
            "Rebuilt {} page(s): {:?}",
            result.pages_rebuilt.len(),
            result.pages_rebuilt
        );
    }

    if !result.dpi_warnings.is_empty() {
        warn!(
            "\nWARNING: {} photo(s) below 300 DPI:",
            result.dpi_warnings.len()
        );
        for w in &result.dpi_warnings {
            warn!(
                "  Page {}: {} — {:.0} DPI",
                w.page, w.photo_id, w.actual_dpi
            );
        }
    }
}
