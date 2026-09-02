//! `fotobuch build` command
mod build_layout;
pub mod errors;
mod helpers;
pub mod plan;

pub use errors::BuildError;
pub use plan::BuildPlan;

use crate::state_manager::StateManager;
use anyhow::Result;
use std::path::{Path, PathBuf};

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
