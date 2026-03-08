//! Final cache generation for high-quality PDF output

use anyhow::Result;
use crate::commands::build::DpiWarning;
use crate::dto_models::ProjectState;
use std::path::Path;
use std::sync::atomic::AtomicUsize;

/// Result of final cache generation
#[derive(Debug)]
pub struct FinalCacheResult {
    /// Number of images created
    pub created: usize,
    /// DPI warnings for images below 300 DPI
    pub dpi_warnings: Vec<DpiWarning>,
}

/// Builds final cache from original images at 300 DPI.
/// TODO: Implementation in later commit
pub fn build_final_cache(
    _state: &ProjectState,
    _final_cache_dir: &Path,
    _progress: &AtomicUsize,
) -> Result<FinalCacheResult> {
    Ok(FinalCacheResult {
        created: 0,
        dpi_warnings: Vec::new(),
    })
}
