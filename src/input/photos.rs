//! Photo loading and metadata extraction.

use crate::model::Photo;
use crate::scanner;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Photo information including path and metadata.
#[derive(Debug, Clone)]
pub struct PhotoInfo {
    pub path: PathBuf,
    pub photo: Photo,
}

/// Loads photos from a directory using the scanner module.
///
/// Returns a vector of photos with metadata and their file paths.
pub fn load_photos_from_dir(dir: &Path) -> Result<Vec<PhotoInfo>> {
    let groups = scanner::scan_photo_dirs(dir)?;
    
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
            };
            
            photo_infos.push(PhotoInfo {
                path: scanned_photo.path,
                photo,
            });
        }
    }
    
    Ok(photo_infos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_photo_info_creation() {
        let photo = Photo::new(1.5, 1.0, "test".to_string());
        let info = PhotoInfo {
            path: PathBuf::from("test.jpg"),
            photo,
        };
        
        assert_eq!(info.photo.aspect_ratio, 1.5);
        assert_eq!(info.path, PathBuf::from("test.jpg"));
    }
}
