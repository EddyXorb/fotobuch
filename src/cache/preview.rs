//! Preview image cache generation

use anyhow::Result;
use crate::cache::common::{is_cache_fresh, preview_path, resize_and_save};
use crate::dto_models::{PhotoFile, ProjectState};
use rayon::prelude::*;
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
/// Generates missing or stale previews in parallel using rayon.
/// Updates progress counter atomically as each image is processed.
///
/// # Arguments
/// * `state` - Current project state with photo groups
/// * `preview_cache_dir` - Base directory for preview cache (from StateManager)
/// * `progress` - Atomic counter incremented for each processed image
///
/// # Returns
/// Statistics about created/skipped images
pub fn ensure_previews(
    state: &ProjectState,
    preview_cache_dir: &Path,
    progress: &AtomicUsize,
) -> Result<PreviewCacheResult> {
    let max_px = state.config.preview.max_preview_px;

    // Collect all (group, photo) pairs
    let all_photos: Vec<(&str, &PhotoFile)> = state
        .photos
        .iter()
        .flat_map(|g| g.files.iter().map(move |f| (g.group.as_str(), f)))
        .collect();

    let total = all_photos.len();
    let created = AtomicUsize::new(0);
    let skipped = AtomicUsize::new(0);

    // Process in parallel
    all_photos.par_iter().try_for_each(|(group, photo)| {
        let source = Path::new(&photo.source);
        let cached = preview_path(preview_cache_dir, group, &photo.id);

        if is_cache_fresh(source, &cached) {
            skipped.fetch_add(1, Ordering::Relaxed);
        } else {
            let (target_width, target_height) = fit_dimensions(
                photo.width_px,
                photo.height_px,
                max_px,
            );
            resize_and_save(source, &cached, target_width, target_height, 85)?;
            created.fetch_add(1, Ordering::Relaxed);
        }

        progress.fetch_add(1, Ordering::Relaxed);
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(PreviewCacheResult {
        created: created.load(Ordering::Relaxed),
        skipped: skipped.load(Ordering::Relaxed),
        total,
    })
}

/// Calculates target dimensions so that the longest edge equals max_px.
/// Maintains aspect ratio.
fn fit_dimensions(width: u32, height: u32, max_px: u32) -> (u32, u32) {
    if width == 0 || height == 0 {
        return (max_px, max_px);
    }

    if width >= height {
        // Width is longest edge
        let target_width = max_px;
        let target_height = (height as f64 * max_px as f64 / width as f64).round() as u32;
        (target_width, target_height.max(1))
    } else {
        // Height is longest edge
        let target_height = max_px;
        let target_width = (width as f64 * max_px as f64 / height as f64).round() as u32;
        (target_width.max(1), target_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::*;
    use chrono::Utc;
    use image::ImageFormat;
    use std::{fs, thread};
    use std::time::Duration;
    use tempfile::TempDir;

    fn create_test_image(path: &Path, width: u32, height: u32) {
        let img = image::RgbImage::from_fn(width, height, |_, _| image::Rgb([255, 0, 0]));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        img.save_with_format(path, ImageFormat::Jpeg).unwrap();
    }

    fn create_test_config(max_preview_px: u32) -> ProjectConfig {
        ProjectConfig {
            book: BookConfig {
                title: "Test Book".to_string(),
                page_width_mm: 297.0,
                page_height_mm: 210.0,
                bleed_mm: 3.0,
                margin_mm: 10.0,
                gap_mm: 5.0,
                bleed_threshold_mm: 3.0,
            },
            page_layout_solver: GaConfig::default(),
            preview: PreviewConfig {
                max_preview_px,
                ..Default::default()
            },
            book_layout_solver: BookLayoutSolverConfig::default(),
        }
    }

    #[test]
    fn test_fit_dimensions_landscape() {
        let (w, h) = fit_dimensions(1920, 1080, 800);
        assert_eq!(w, 800);
        assert_eq!(h, 450);
    }

    #[test]
    fn test_fit_dimensions_portrait() {
        let (w, h) = fit_dimensions(1080, 1920, 800);
        assert_eq!(w, 450);
        assert_eq!(h, 800);
    }

    #[test]
    fn test_fit_dimensions_square() {
        let (w, h) = fit_dimensions(1000, 1000, 800);
        assert_eq!(w, 800);
        assert_eq!(h, 800);
    }

    #[test]
    fn test_fit_dimensions_zero() {
        let (w, h) = fit_dimensions(0, 0, 800);
        assert_eq!(w, 800);
        assert_eq!(h, 800);
    }

    #[test]
    fn test_ensure_previews_creates_missing() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path();
        let photos_dir = project_root.join("photos");
        let cache_dir = project_root.join("cache/preview");

        // Create source image
        let source_path = photos_dir.join("test.jpg");
        create_test_image(&source_path, 1920, 1080);

        // Create test state
        let state = ProjectState {
            config: create_test_config(400),
            photos: vec![PhotoGroup {
                group: "TestGroup".to_string(),
                sort_key: "01".to_string(),
                files: vec![PhotoFile {
                    id: "TestGroup_001".to_string(),
                    source: source_path.to_str().unwrap().to_string(),
                    width_px: 1920,
                    height_px: 1080,
                    area_weight: 1.0,
                    timestamp: Utc::now(),
                    hash: String::new(),
                }],
            }],
            layout: vec![],
        };

        let progress = AtomicUsize::new(0);
        let result = ensure_previews(&state, &cache_dir, &progress).unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.created, 1);
        assert_eq!(result.skipped, 0);
        assert_eq!(progress.load(Ordering::Relaxed), 1);

        // Verify cached image exists and has correct dimensions
        let cached_path = cache_dir.join("TestGroup/001.jpg");
        assert!(cached_path.exists());
        let cached_img = image::open(&cached_path).unwrap();
        assert_eq!(cached_img.width(), 400);
        assert_eq!(cached_img.height(), 225);
    }

    #[test]
    fn test_ensure_previews_skips_fresh() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path();
        let photos_dir = project_root.join("photos");
        let cache_dir = project_root.join("cache/preview");

        // Create source and cached images
        let source_path = photos_dir.join("test.jpg");
        create_test_image(&source_path, 1920, 1080);

        thread::sleep(Duration::from_millis(10));

        let cached_path = cache_dir.join("TestGroup/001.jpg");
        create_test_image(&cached_path, 400, 225);

        // Create test state
        let state = ProjectState {
            config: create_test_config(400),
            photos: vec![PhotoGroup {
                group: "TestGroup".to_string(),
                sort_key: "01".to_string(),
                files: vec![PhotoFile {
                    id: "TestGroup_001".to_string(),
                    source: source_path.to_str().unwrap().to_string(),
                    width_px: 1920,
                    height_px: 1080,
                    area_weight: 1.0,
                    timestamp: Utc::now(),
                    hash: String::new(),
                }],
            }],
            layout: vec![],
        };

        let progress = AtomicUsize::new(0);
        let result = ensure_previews(&state, &cache_dir, &progress).unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.created, 0);
        assert_eq!(result.skipped, 1);
    }

    #[test]
    fn test_ensure_previews_updates_stale() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path();
        let photos_dir = project_root.join("photos");
        let cache_dir = project_root.join("cache/preview");

        // Create cached image first (older)
        let cached_path = cache_dir.join("TestGroup/001.jpg");
        create_test_image(&cached_path, 400, 225);

        thread::sleep(Duration::from_millis(10));

        // Create source image (newer)
        let source_path = photos_dir.join("test.jpg");
        create_test_image(&source_path, 1920, 1080);

        let state = ProjectState {
            config: create_test_config(400),
            photos: vec![PhotoGroup {
                group: "TestGroup".to_string(),
                sort_key: "01".to_string(),
                files: vec![PhotoFile {
                    id: "TestGroup_001".to_string(),
                    source: source_path.to_str().unwrap().to_string(),
                    width_px: 1920,
                    height_px: 1080,
                    area_weight: 1.0,
                    timestamp: Utc::now(),
                    hash: String::new(),
                }],
            }],
            layout: vec![],
        };

        let progress = AtomicUsize::new(0);
        let result = ensure_previews(&state, &cache_dir, &progress).unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.created, 1);
        assert_eq!(result.skipped, 0);
    }
}
