use std::path::PathBuf;
use super::photo::Photo;

/// Bridge between scanned photos (with file paths) and solver photos (with optimization data).
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
    use super::*;

    #[test]
    fn test_photo_info_creation() {
        let photo = Photo::new(1.5, 1.0, "test".to_string());
        let info = PhotoInfo::new(PathBuf::from("test.jpg"), photo);
        
        assert_eq!(info.photo.aspect_ratio, 1.5);
        assert_eq!(info.path, PathBuf::from("test.jpg"));
    }
}
