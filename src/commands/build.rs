//! `fotobuch build` command
pub(super) mod cover;
mod helpers;
pub mod plan;
mod rebuild_single_page;

pub use plan::BuildPlan;

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

/// Configuration for the `build` command.
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// Which layout strategy to apply.
    pub plan: BuildPlan,
    /// Skip PDF generation (Typst compilation). Only produce YAML layout.
    pub skip_pdf: bool,
    /// Skip preview cache update. Use with caution.
    pub skip_cache_update: bool,
}

/// Result of build command
#[derive(Debug)]
pub struct BuildResult {
    /// Path to generated PDF
    pub pdf_path: PathBuf,
    /// Pages that were rebuilt (0-based array indices into layout[])
    pub pages_rebuilt: Vec<usize>,
    /// Number of images processed in cache
    pub images_processed: usize,
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
    let skip_pdf = config.skip_pdf || !mgr.state.config.preview.write_pdf;
    config
        .plan
        .clone()
        .run(mgr, skip_pdf, config.skip_cache_update)
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
