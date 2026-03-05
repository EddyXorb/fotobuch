//! Photo loading and metadata extraction.

use crate::{
    input::scanner::scan_photo_dirs,
    models::{Photo, PhotoInfo},
};
use anyhow::Result;
use std::path::Path;

/// Loads photos from a directory using the scanner module.
///
/// Returns a vector of photos with metadata and their file paths.
pub fn load_photos_from_dir(dir: &Path) -> Result<Vec<PhotoInfo>> {
    let groups = scan_photo_dirs(dir)?;

    let mut photo_infos = Vec::new();

    for group in groups {
        let group_name = group.label.clone();

        for scanned_photo in group.photos {
            // Extract aspect ratio from dimensions
            let aspect_ratio = if let Some((w, h)) = scanned_photo.dimensions {
                w as f64 / h as f64
            } else {
                // Default to square if dimensions are unknown
                1.0
            };

            // Convert NaiveDateTime to DateTime<Utc> if available
            let timestamp = scanned_photo.timestamp.map(|naive| {
                use chrono::{DateTime, Utc};
                DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
            });

            let photo = Photo {
                aspect_ratio,
                area_weight: 1.0, // Default weight
                group: group_name.clone(),
                timestamp,
                dimensions: scanned_photo.dimensions,
            };

            photo_infos.push(PhotoInfo::new(scanned_photo.path, photo));
        }
    }

    Ok(photo_infos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_photo_info_creation() {
        let photo = Photo::new(1.5, 1.0, "test".to_string());
        let info = PhotoInfo::new(PathBuf::from("test.jpg"), photo);

        assert_eq!(info.photo.aspect_ratio, 1.5);
        assert_eq!(info.path, PathBuf::from("test.jpg"));
    }
}
