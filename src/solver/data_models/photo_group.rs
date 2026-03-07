use chrono::NaiveDateTime;
use std::path::PathBuf;

/// A single photo file with filesystem and EXIF metadata (used by scanner).
#[derive(Debug, Clone)]
pub struct ScannedPhoto {
    /// File path to the photo.
    pub path: PathBuf,

    /// Timestamp from EXIF data, or derived from the folder name as fallback.
    pub timestamp: Option<NaiveDateTime>,

    /// Pixel dimensions (width, height), if readable.
    pub dimensions: Option<(u32, u32)>,
}

impl ScannedPhoto {
    /// Creates a new scanned photo with the given path.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            timestamp: None,
            dimensions: None,
        }
    }

    /// Whether the photo is in landscape orientation.
    pub fn is_landscape(&self) -> bool {
        self.dimensions.map(|(w, h)| w >= h).unwrap_or(true)
    }

    /// Returns the aspect ratio (width / height), or a default if unknown.
    pub fn aspect_ratio(&self) -> f64 {
        self.dimensions
            .map(|(w, h)| w as f64 / h as f64)
            .unwrap_or(1.5)
    }
}

/// A group of photos that belong together (e.g. from the same folder/day).
/// Each group will typically be laid out on a single page in the photobook.
#[derive(Debug)]
pub struct PhotoGroup {
    /// Group label (typically the folder name).
    pub label: String,

    /// Group timestamp (typically parsed from folder name).
    pub timestamp: Option<NaiveDateTime>,

    /// Photos in this group.
    pub photos: Vec<ScannedPhoto>,
}

impl PhotoGroup {
    /// Creates a new photo group.
    pub fn new(label: String, timestamp: Option<NaiveDateTime>) -> Self {
        Self {
            label,
            timestamp,
            photos: Vec::new(),
        }
    }

    /// Adds a photo to the group.
    pub fn add_photo(&mut self, photo: ScannedPhoto) {
        self.photos.push(photo);
    }

    /// Returns the number of photos in the group.
    pub fn len(&self) -> usize {
        self.photos.len()
    }

    /// Returns whether the group is empty.
    pub fn is_empty(&self) -> bool {
        self.photos.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanned_photo_new() {
        let photo = ScannedPhoto::new(PathBuf::from("test.jpg"));
        assert_eq!(photo.path, PathBuf::from("test.jpg"));
        assert!(photo.timestamp.is_none());
        assert!(photo.dimensions.is_none());
    }

    #[test]
    fn test_scanned_photo_is_landscape() {
        let mut photo = ScannedPhoto::new(PathBuf::from("test.jpg"));
        photo.dimensions = Some((1920, 1080));
        assert!(photo.is_landscape());
    }

    #[test]
    fn test_scanned_photo_aspect_ratio() {
        let mut photo = ScannedPhoto::new(PathBuf::from("test.jpg"));
        photo.dimensions = Some((1920, 1080));
        assert!((photo.aspect_ratio() - 1.777777).abs() < 0.001);
    }

    #[test]
    fn test_photo_group_new() {
        let group = PhotoGroup::new("test_group".to_string(), None);
        assert_eq!(group.label, "test_group");
        assert!(group.timestamp.is_none());
        assert!(group.is_empty());
    }

    #[test]
    fn test_photo_group_add_photo() {
        let mut group = PhotoGroup::new("test_group".to_string(), None);
        let photo = ScannedPhoto::new(PathBuf::from("test.jpg"));
        group.add_photo(photo);
        assert_eq!(group.len(), 1);
        assert!(!group.is_empty());
    }
}
