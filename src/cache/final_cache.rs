//! Final cache generation for high-quality PDF output

use anyhow::Result;
use crate::commands::build::DpiWarning;
use crate::dto_models::ProjectState;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use rayon::prelude::*;

use super::common::{cache_rel_path, is_cache_fresh, resize_and_save};

/// Result of final cache generation
#[derive(Debug)]
pub struct FinalCacheResult {
    /// Number of images created
    pub created: usize,
    /// DPI warnings for images below 300 DPI
    pub dpi_warnings: Vec<DpiWarning>,
}

/// Builds final cache from original images at 300 DPI.
///
/// Processes all photos in the layout, generating cached images at 300 DPI
/// for each slot. Collects warnings for photos that will be displayed below 300 DPI.
pub fn build_final_cache(
    state: &ProjectState,
    final_cache_dir: &Path,
    progress: &AtomicUsize,
) -> Result<FinalCacheResult> {
    const TARGET_DPI: f64 = 300.0;

    // Build photo lookup map: photo_id -> PhotoFile
    let photo_map: HashMap<&str, &crate::dto_models::PhotoFile> = state
        .photos
        .iter()
        .flat_map(|g| g.files.iter().map(|f| (f.id.as_str(), f)))
        .collect();

    // Collect all (page_num, slot_index, photo_id) tuples
    let mut tasks = Vec::new();
    for page in &state.layout {
        for (idx, photo_id) in page.photos.iter().enumerate() {
            if let Some(slot) = page.slots.get(idx)
                && let Some(&photo) = photo_map.get(photo_id.as_str()) {
                    tasks.push((page.page, slot.clone(), photo, photo_id.clone()));
                }
        }
    }

    // Track progress and warnings
    let created = AtomicUsize::new(0);
    let warnings = std::sync::Mutex::new(Vec::new());

    // Process in parallel
    tasks.par_iter().try_for_each(|(page_num, slot, photo, photo_id)| {
        let source = Path::new(&photo.source);
        let cached = final_cache_dir.join(cache_rel_path(photo_id));

        // Calculate target dimensions at 300 DPI
        let (target_w, target_h) = target_pixels(slot, TARGET_DPI);

        // Check if photo meets DPI requirements
        let photo_dpi = actual_dpi(photo.width_px, photo.height_px, slot);
        if photo_dpi < TARGET_DPI {
            warnings.lock().unwrap().push(DpiWarning {
                page: *page_num,
                photo_id: photo_id.clone(),
                actual_dpi: photo_dpi,
                original_px: (photo.width_px, photo.height_px),
                slot_mm: (slot.width_mm, slot.height_mm),
            });
        }

        // Generate final image if missing or stale
        if !is_cache_fresh(source, &cached) {
            resize_and_save(source, &cached, target_w, target_h, 95)?;
            created.fetch_add(1, Ordering::Relaxed);
        }

        progress.fetch_add(1, Ordering::Relaxed);
        Ok::<_, anyhow::Error>(())
    })?;

    Ok(FinalCacheResult {
        created: created.into_inner(),
        dpi_warnings: warnings.into_inner().unwrap(),
    })
}

/// Calculates target pixel dimensions from slot size (mm) and DPI.
///
/// Formula: pixels = (mm / 25.4) * dpi
fn target_pixels(slot: &crate::dto_models::Slot, dpi: f64) -> (u32, u32) {
    let w = (slot.width_mm / 25.4 * dpi).round() as u32;
    let h = (slot.height_mm / 25.4 * dpi).round() as u32;
    (w, h)
}

/// Calculates the actual DPI a photo will be displayed at in a slot.
///
/// Returns the minimum of horizontal and vertical DPI (limiting factor).
fn actual_dpi(photo_width_px: u32, photo_height_px: u32, slot: &crate::dto_models::Slot) -> f64 {
    let dpi_w = photo_width_px as f64 / (slot.width_mm / 25.4);
    let dpi_h = photo_height_px as f64 / (slot.height_mm / 25.4);
    dpi_w.min(dpi_h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::Slot;

    #[test]
    fn test_target_pixels_at_300dpi() {
        let slot = Slot {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: 100.0,  // 100mm ~= 3.937 inches
            height_mm: 50.0,  // 50mm ~= 1.969 inches
        };

        let (w, h) = target_pixels(&slot, 300.0);
        // 100mm / 25.4 * 300 = 1181.1 -> 1181
        // 50mm / 25.4 * 300 = 590.55 -> 591
        assert_eq!(w, 1181);
        assert_eq!(h, 591);
    }

    #[test]
    fn test_actual_dpi_sufficient() {
        let slot = Slot {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: 100.0,
            height_mm: 50.0,
        };

        // Photo at exactly 300 DPI
        let dpi = actual_dpi(1181, 591, &slot);
        assert!((dpi - 300.0).abs() < 1.0);
    }

    #[test]
    fn test_actual_dpi_insufficient() {
        let slot = Slot {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: 100.0,
            height_mm: 50.0,
        };

        // Photo too small (72 DPI equivalent)
        let dpi = actual_dpi(283, 142, &slot);
        assert!((dpi - 72.0).abs() < 1.0);
    }

    #[test]
    fn test_actual_dpi_limiting_factor() {
        let slot = Slot {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: 100.0,
            height_mm: 50.0,
        };

        // Width sufficient (300 DPI), height insufficient (150 DPI)
        let dpi = actual_dpi(1181, 295, &slot);
        assert!((dpi - 150.0).abs() < 1.0);
    }
}
