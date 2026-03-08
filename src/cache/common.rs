//! Common cache utilities for preview and final images

use anyhow::{Context, Result};
use image::imageops::FilterType;
use std::fs;
use std::path::{Path, PathBuf};

/// Derives the relative cache path from a photo ID.
///
/// The photo ID is already a relative path including the file extension
/// (e.g. `"2024-01-15_Urlaub/IMG_001.jpg"`), so this function returns it as-is.
/// The Typst template references images with `cache_prefix + photo_id`.
///
/// # Examples
/// ```
/// use photobook_solver::cache::common::cache_rel_path;
/// let path = cache_rel_path("Urlaub/IMG_001.jpg");
/// assert_eq!(path.to_str().unwrap(), "Urlaub/IMG_001.jpg");
/// ```
pub fn cache_rel_path(photo_id: &str) -> PathBuf {
    PathBuf::from(photo_id)
}

/// Returns absolute preview cache path for a photo.
/// `cache_base` should come from `StateManager::preview_cache_dir()`.
pub fn preview_path(cache_base: &Path, photo_id: &str) -> PathBuf {
    cache_base.join(cache_rel_path(photo_id))
}

/// Returns absolute final cache path for a photo.
/// `cache_base` should come from `StateManager::final_cache_dir()`.
pub fn final_path(cache_base: &Path, photo_id: &str) -> PathBuf {
    cache_base.join(cache_rel_path(photo_id))
}

/// Checks if cached image is fresh (exists and newer than source).
/// Returns true if cached exists and has mtime >= source mtime.
pub fn is_cache_fresh(source: &Path, cached: &Path) -> bool {
    if !cached.exists() {
        return false;
    }

    // Compare modification times
    match (fs::metadata(source), fs::metadata(cached)) {
        (Ok(src_meta), Ok(cache_meta)) => {
            if let (Ok(src_time), Ok(cache_time)) = (src_meta.modified(), cache_meta.modified()) {
                cache_time >= src_time
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Resizes image and saves as JPEG with specified quality.
/// Automatically chooses filter: Lanczos3 for downscaling ≤2x, Triangle for >2x.
/// Creates parent directories if they don't exist.
///
/// # Arguments
/// * `source` - Path to source image
/// * `target` - Path to save resized image
/// * `target_width` - Target width in pixels
/// * `target_height` - Target height in pixels
/// * `jpeg_quality` - JPEG quality (0-100)
pub fn resize_and_save(
    source: &Path,
    target: &Path,
    target_width: u32,
    target_height: u32,
    jpeg_quality: u8,
) -> Result<()> {
    // Load image
    let img = image::open(source)
        .with_context(|| format!("Failed to open image: {}", source.display()))?;

    // Calculate scale factor
    let scale_x = img.width() as f64 / target_width as f64;
    let scale_y = img.height() as f64 / target_height as f64;
    let scale = scale_x.max(scale_y);

    // Choose filter based on scale factor
    let filter = if scale <= 2.0 {
        FilterType::Lanczos3 // Better quality for moderate downscaling
    } else {
        FilterType::Triangle // Faster for large downscaling
    };

    // Resize
    let resized = img.resize(target_width, target_height, filter);

    // Create parent directory if needed
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Save as JPEG
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
        fs::File::create(target)
            .with_context(|| format!("Failed to create file: {}", target.display()))?,
        jpeg_quality,
    );

    encoder
        .encode(
            resized.as_bytes(),
            resized.width(),
            resized.height(),
            resized.color().into(), // Convert ColorType to ExtendedColorType
        )
        .with_context(|| format!("Failed to encode JPEG: {}", target.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageFormat;
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_cache_rel_path() {
        let path = cache_rel_path("Urlaub/IMG_001.jpg");
        assert_eq!(path.to_str().unwrap(), "Urlaub/IMG_001.jpg");
    }

    #[test]
    fn test_cache_rel_path_with_suffix() {
        let path = cache_rel_path("Urlaub/IMG_001_1.jpg");
        assert_eq!(path.to_str().unwrap(), "Urlaub/IMG_001_1.jpg");
    }

    #[test]
    fn test_preview_path() {
        let base = Path::new("/cache/preview");
        let path = preview_path(base, "Urlaub/IMG_001.jpg");
        assert_eq!(
            path.to_str().unwrap(),
            "/cache/preview/Urlaub/IMG_001.jpg"
        );
    }

    #[test]
    fn test_final_path() {
        let base = Path::new("/cache/final");
        let path = final_path(base, "Urlaub/IMG_001.jpg");
        assert_eq!(path.to_str().unwrap(), "/cache/final/Urlaub/IMG_001.jpg");
    }

    #[test]
    fn test_is_cache_fresh_missing_cache() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.jpg");
        let cached = temp.path().join("cached.jpg");

        fs::write(&source, b"test").unwrap();

        assert!(!is_cache_fresh(&source, &cached));
    }

    #[test]
    fn test_is_cache_fresh_newer_cache() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.jpg");
        let cached = temp.path().join("cached.jpg");

        fs::write(&source, b"test").unwrap();
        thread::sleep(Duration::from_millis(10));
        fs::write(&cached, b"cached").unwrap();

        assert!(is_cache_fresh(&source, &cached));
    }

    #[test]
    fn test_is_cache_fresh_older_cache() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.jpg");
        let cached = temp.path().join("cached.jpg");

        fs::write(&cached, b"cached").unwrap();
        thread::sleep(Duration::from_millis(10));
        fs::write(&source, b"test").unwrap();

        assert!(!is_cache_fresh(&source, &cached));
    }

    #[test]
    fn test_resize_and_save() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.jpg");
        let target = temp.path().join("subdir").join("target.jpg");

        // Create a test image (100x100 red square)
        let img = image::RgbImage::from_fn(100, 100, |_, _| image::Rgb([255, 0, 0]));
        img.save_with_format(&source, ImageFormat::Jpeg).unwrap();

        // Resize to 50x50
        resize_and_save(&source, &target, 50, 50, 85).unwrap();

        // Verify target exists and has correct dimensions
        assert!(target.exists());
        let resized = image::open(&target).unwrap();
        assert_eq!(resized.width(), 50);
        assert_eq!(resized.height(), 50);
    }
}
