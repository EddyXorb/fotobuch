use crate::models::PhotoFile;

/// Photo model for the layout solver with optimization metadata.
#[derive(Debug, Clone)]
pub struct Photo {
    /// Unique photo identifier.
    pub id: String,

    /// Aspect ratio: width / height.
    pub aspect_ratio: f64,

    /// Relative importance for size distribution (default: 1.0).
    /// Higher values → photo should get more area.
    pub area_weight: f64,

    /// Group identifier (e.g., folder name, event).
    pub group: String,
}

impl Photo {
    /// Creates a new photo with the given parameters.
    #[allow(dead_code)]
    pub fn new(id: String, aspect_ratio: f64, area_weight: f64, group: String) -> Self {
        assert!(aspect_ratio > 0.0, "Aspect ratio must be positive");
        assert!(area_weight > 0.0, "Area weight must be positive");

        Self {
            id,
            aspect_ratio,
            area_weight,
            group,
        }
    }

    /// Returns whether the photo is in landscape orientation (width >= height).
    #[allow(dead_code)]
    pub fn is_landscape(&self) -> bool {
        self.aspect_ratio >= 1.0
    }

    /// Returns whether the photo is in portrait orientation (height > width).
    #[allow(dead_code)]
    pub fn is_portrait(&self) -> bool {
        self.aspect_ratio < 1.0
    }

    /// Converts a PhotoFile to a Photo with explicit group name.
    ///
    /// # Arguments
    ///
    /// * `file` - PhotoFile from DTO layer
    /// * `group` - Group identifier (e.g., folder name)
    ///
    /// # Returns
    ///
    /// A new Photo instance with data from PhotoFile
    pub fn from_photo_file(file: &PhotoFile, group: &str) -> Self {
        Self {
            id: file.id.clone(),
            aspect_ratio: file.aspect_ratio(),
            area_weight: file.area_weight,
            group: group.to_string(),
        }
    }
}

// /// Bridge between scanned photos (with file paths) and solver photos (with optimization data).
// ///
// /// Combines file system information with solver-ready photo metadata.
// #[derive(Debug, Clone)]
// pub struct PhotoInfo {
//     /// File path to the photo.
//     pub path: PathBuf,

//     /// Solver-ready photo with aspect ratio and optimization metadata.
//     pub photo: Photo,
// }

// impl PhotoInfo {
//     /// Creates a new PhotoInfo.
//     pub fn new(path: PathBuf, photo: Photo) -> Self {
//         Self { path, photo }
//     }
// }

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
    }

    #[test]
    #[should_panic(expected = "Aspect ratio must be positive")]
    fn test_new_photo_negative_aspect_ratio() {
        Photo::new(
            "id".to_string(),
            -1.0,
            DEFAULT_AREA_WEIGHT,
            "test".to_string(),
        );
    }

    #[test]
    #[should_panic(expected = "Area weight must be positive")]
    fn test_new_photo_negative_area_weight() {
        Photo::new("id".to_string(), LANDSCAPE_ASPECT, -1.0, "test".to_string());
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
}
