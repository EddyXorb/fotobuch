//! Preview image cache generation

use anyhow::Result;
use crate::dto_models::{PhotoFile, ProjectState};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Result of preview cache generation
#[derive(Debug)]
pub struct PreviewCacheResult {
    /// Number of images created
    pub created: usize,
    /// Number of images skipped (already fresh)
    pub skipped: usize,
    /// Total number of images processed
    pub total: usize,
}

/// Ensures all preview images are present and up-to-date.
/// TODO: Implementation in next commit
pub fn ensure_previews(
    _state: &ProjectState,
    _preview_cache_dir: &Path,
    _progress: &AtomicUsize,
) -> Result<PreviewCacheResult> {
    Ok(PreviewCacheResult {
        created: 0,
        skipped: 0,
        total: 0,
    })
}
