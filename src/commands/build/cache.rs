use anyhow::Result;
use std::sync::atomic::AtomicUsize;
use tracing::{info, warn};

use super::DpiWarning;
use crate::cache::{final_cache, preview_cache};
use crate::state_manager::StateManager;

pub fn refresh_preview_cache(
    mgr: &mut StateManager,
) -> Result<preview_cache::PreviewCacheResult, anyhow::Error> {
    let preview_cache_dir = mgr.preview_cache_dir();
    let cache_result = preview_cache::build_preview_cache(&mut mgr.state, &preview_cache_dir)?;
    if cache_result.created > 0 {
        info!(
            "Preview cache: {} created, {} skipped",
            cache_result.created, cache_result.skipped
        );
    };
    Ok(cache_result)
}

/// Outcome of a cache refresh: how many images were (re)generated and any DPI
/// warnings raised while building the final cache.
pub struct CacheRefresh {
    pub images_processed: usize,
    pub dpi_warnings: Vec<DpiWarning>,
}

impl CacheRefresh {
    /// A refresh that only (re)generated images and cannot carry DPI warnings
    /// (preview builds and skipped refreshes).
    pub fn images_only(images_processed: usize) -> Self {
        Self {
            images_processed,
            dpi_warnings: Vec::new(),
        }
    }
}

/// Builds the final high-resolution image cache for a release build and logs DPI
/// warnings.
pub fn refresh_final_cache(mgr: &mut StateManager) -> Result<CacheRefresh> {
    let dpi = mgr.state.config.book.dpi;
    info!("Release build: generating final PDF at {:.0} DPI...", dpi);

    let progress = AtomicUsize::new(0);
    let final_cache_dir = mgr.final_cache_dir();
    let result = final_cache::build_final_cache(&mut mgr.state, &final_cache_dir, &progress)?;

    info!(
        "Final cache: {} images generated, {} DPI warnings",
        result.created,
        result.dpi_warnings.len()
    );
    log_dpi_warnings(dpi, &result.dpi_warnings);

    Ok(CacheRefresh {
        images_processed: result.created,
        dpi_warnings: result.dpi_warnings,
    })
}

/// Logs each photo that will be rendered below the target DPI.
fn log_dpi_warnings(target_dpi: f64, warnings: &[DpiWarning]) {
    if warnings.is_empty() {
        return;
    }
    warn!(
        "\nWARNING: Some photos will be displayed below {:.0} DPI:",
        target_dpi
    );
    for w in warnings {
        warn!(
            "  Page {}: {} - {:.2} DPI ({}x{} px in {:.1}x{:.1} mm slot)",
            w.page,
            w.photo_id,
            w.actual_dpi,
            w.original_px.0,
            w.original_px.1,
            w.slot_mm.0,
            w.slot_mm.1
        );
    }
}
