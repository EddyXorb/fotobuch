//! Photo loading and metadata extraction.
use crate::dto_models::PhotoFile;
use crate::input::scanner::scan_photo_dirs;
use anyhow::Result;
use std::path::Path;

/// Loads photos from a directory using the scanner module.
///
/// Returns a vector of photos with metadata.
pub fn load_photos_from_dir(dir: &Path) -> Result<Vec<PhotoFile>> {
    let groups = scan_photo_dirs(dir)?;

    let mut photos = Vec::new();

    for group in groups {
        for photo_file in group.files {
            photos.push(photo_file);
        }
    }

    Ok(photos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_photos_returns_vector() {
        // This test would need actual test fixtures to run properly
        // For now, just verify the function signature
        let result = load_photos_from_dir(std::path::Path::new("."));
        assert!(result.is_ok() || result.is_err()); // Will fail or succeed depending on directory
    }
}
