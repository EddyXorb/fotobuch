use chrono::{DateTime, Utc};

use crate::dto_models::{PhotoFile, PhotoGroup};

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

    /// Timestamp from EXIF or folder name.
    pub timestamp: Option<DateTime<Utc>>,

    /// Absolute pixel dimensions (width, height).
    pub dimensions: Option<(u32, u32)>,
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
            timestamp: None,
            dimensions: None,
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
            timestamp: Some(file.timestamp),
            dimensions: Some((file.width_px, file.height_px)),
        }
    }

    /// Converts a slice of PhotoGroups to a Vec of Photos.
    ///
    /// Flattens all PhotoFiles from all groups into a single vector.
    /// Each photo gets its group name from the containing PhotoGroup.
    ///
    /// # Arguments
    ///
    /// * `groups` - Slice of PhotoGroups to convert
    ///
    /// # Returns
    ///
    /// A vector of Photos with proper group assignments
    pub fn from_photo_groups(groups: &[PhotoGroup]) -> Vec<Self> {
        groups
            .iter()
            .flat_map(|group| {
                group
                    .files
                    .iter()
                    .map(|file| Self::from_photo_file(file, &group.group))
            })
            .collect()
    }
}

/// Bridge between scanned photos (with file paths) and solver photos (with optimization data).
///
/// Combines file system information with solver-ready photo metadata.
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
        assert!(photo.timestamp.is_none());
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

    // Converter tests
    mod converter_tests {
        use super::*;
        use chrono::Utc;

        fn create_photo_file(id: &str, width: u32, height: u32) -> PhotoFile {
            PhotoFile {
                id: id.to_string(),
                source: format!("test/{}.jpg", id),
                width_px: width,
                height_px: height,
                area_weight: 1.0,
                timestamp: Utc::now(),
                hash: String::new(),
            }
        }

        #[test]
        fn test_from_photo_file() {
            let file = create_photo_file("photo1", 1500, 1000);
            let photo = Photo::from_photo_file(&file, "vacation");

            assert_eq!(photo.id, "photo1");
            assert_eq!(photo.aspect_ratio, 1.5);
            assert_eq!(photo.area_weight, 1.0);
            assert_eq!(photo.group, "vacation");
            assert!(photo.timestamp.is_some());
            assert_eq!(photo.dimensions, Some((1500, 1000)));
        }

        #[test]
        fn test_from_photo_file_portrait() {
            let file = create_photo_file("photo2", 1000, 1500);
            let photo = Photo::from_photo_file(&file, "portraits");

            assert_eq!(photo.id, "photo2");
            assert_eq!(photo.aspect_ratio, 1000.0 / 1500.0);
            assert!(photo.is_portrait());
        }

        #[test]
        fn test_from_photo_groups_empty() {
            let groups: Vec<PhotoGroup> = vec![];
            let photos = Photo::from_photo_groups(&groups);

            assert_eq!(photos.len(), 0);
        }

        #[test]
        fn test_from_photo_groups_single_group() {
            let group = PhotoGroup {
                group: "vacation".to_string(),
                sort_key: "2024-01-01".to_string(),
                files: vec![
                    create_photo_file("p1", 1500, 1000),
                    create_photo_file("p2", 1000, 1500),
                ],
            };

            let photos = Photo::from_photo_groups(&[group]);

            assert_eq!(photos.len(), 2);
            assert_eq!(photos[0].id, "p1");
            assert_eq!(photos[0].group, "vacation");
            assert_eq!(photos[1].id, "p2");
            assert_eq!(photos[1].group, "vacation");
        }

        #[test]
        fn test_from_photo_groups_multiple_groups() {
            let groups = vec![
                PhotoGroup {
                    group: "group1".to_string(),
                    sort_key: "2024-01-01".to_string(),
                    files: vec![
                        create_photo_file("g1p1", 1500, 1000),
                        create_photo_file("g1p2", 1500, 1000),
                    ],
                },
                PhotoGroup {
                    group: "group2".to_string(),
                    sort_key: "2024-01-02".to_string(),
                    files: vec![create_photo_file("g2p1", 1000, 1500)],
                },
            ];

            let photos = Photo::from_photo_groups(&groups);

            assert_eq!(photos.len(), 3);
            assert_eq!(photos[0].id, "g1p1");
            assert_eq!(photos[0].group, "group1");
            assert_eq!(photos[1].id, "g1p2");
            assert_eq!(photos[1].group, "group1");
            assert_eq!(photos[2].id, "g2p1");
            assert_eq!(photos[2].group, "group2");
        }

        #[test]
        fn test_from_photo_groups_preserves_order() {
            let groups = vec![
                PhotoGroup {
                    group: "first".to_string(),
                    sort_key: "2024-01-01".to_string(),
                    files: vec![create_photo_file("p1", 1500, 1000)],
                },
                PhotoGroup {
                    group: "second".to_string(),
                    sort_key: "2024-01-02".to_string(),
                    files: vec![create_photo_file("p2", 1000, 1500)],
                },
            ];

            let photos = Photo::from_photo_groups(&groups);

            // Verify order is preserved: group1 before group2
            assert_eq!(photos[0].group, "first");
            assert_eq!(photos[1].group, "second");
        }
    }
}
