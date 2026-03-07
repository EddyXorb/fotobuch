use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// Photo model for the layout solver with optimization metadata.
#[derive(Debug, Clone)]
pub struct Photo {
    /// Aspect ratio: width / height.
    pub aspect_ratio: f64,

    /// Relative importance for size distribution (default: 1.0).
    /// Higher values → photo should get more area.
    pub area_weight: f64,

    /// Group identifier (e.g., folder name, event).
    pub group: String,

    /// Timestamp from EXIF or folder name.
    pub timestamp: Option<DateTime<Utc>>,

    /// Absolute pixel dimensions (width, height).
    pub dimensions: Option<(u32, u32)>,
}

impl Photo {
    /// Creates a new photo with the given aspect ratio.
    pub fn new(aspect_ratio: f64, area_weight: f64, group: String) -> Self {
        assert!(aspect_ratio > 0.0, "Aspect ratio must be positive");
        assert!(area_weight > 0.0, "Area weight must be positive");

        Self {
            aspect_ratio,
            area_weight,
            group,
            timestamp: None,
            dimensions: None,
        }
    }

    /// Returns whether the photo is in landscape orientation (width >= height).
    pub fn is_landscape(&self) -> bool {
        self.aspect_ratio >= 1.0
    }

    /// Returns whether the photo is in portrait orientation (height > width).
    pub fn is_portrait(&self) -> bool {
        self.aspect_ratio < 1.0
    }
}

/// Bridge between scanned photos (with file paths) and solver photos (with optimization data).
///
/// Combines file system information with solver-ready photo metadata.
#[derive(Debug, Clone)]
pub struct PhotoInfo {
    /// File path to the photo.
    pub path: PathBuf,

    /// Solver-ready photo with aspect ratio and optimization metadata.
    pub photo: Photo,
}

impl PhotoInfo {
    /// Creates a new PhotoInfo.
    pub fn new(path: PathBuf, photo: Photo) -> Self {
        Self { path, photo }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::*;
    use super::*;

    #[test]
    fn test_new_photo() {
        let photo = landscape_photo("test");
        assert_eq!(photo.aspect_ratio, LANDSCAPE_ASPECT);
        assert_eq!(photo.area_weight, DEFAULT_AREA_WEIGHT);
        assert_eq!(photo.group, "test");
        assert!(photo.timestamp.is_none());
    }

    #[test]
    #[should_panic(expected = "Aspect ratio must be positive")]
    fn test_new_photo_negative_aspect_ratio() {
        Photo::new(-1.0, DEFAULT_AREA_WEIGHT, "test".to_string());
    }

    #[test]
    #[should_panic(expected = "Area weight must be positive")]
    fn test_new_photo_negative_area_weight() {
        Photo::new(LANDSCAPE_ASPECT, -1.0, "test".to_string());
    }

    #[test]
    fn test_is_landscape() {
        let landscape = landscape_photo("test");
        assert!(landscape.is_landscape());
        assert!(!landscape.is_portrait());

        let square = square_photo("test");
        assert!(square.is_landscape());
        assert!(!square.is_portrait());
    }

    #[test]
    fn test_is_portrait() {
        let portrait = portrait_photo("test");
        assert!(portrait.is_portrait());
        assert!(!portrait.is_landscape());
    }

    #[test]
    fn test_photo_info_creation() {
        let photo = landscape_photo("test");
        let info = PhotoInfo::new(PathBuf::from("test.jpg"), photo);

        assert_eq!(info.photo.aspect_ratio, LANDSCAPE_ASPECT);
        assert_eq!(info.path, PathBuf::from("test.jpg"));
    }
}
