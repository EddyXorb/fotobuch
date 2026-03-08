//! Group merging logic for photo groups

use crate::dto_models::PhotoGroup;

/// Merges a scanned group into the project photos.
///
/// If a group with the same name already exists, appends the files.
/// Otherwise, adds the group as new.
///
/// # Arguments
/// * `photos` - Mutable reference to the project's photo groups
/// * `scanned` - The scanned group to merge
pub fn merge_group(photos: &mut Vec<PhotoGroup>, scanned: PhotoGroup) {
    if let Some(existing) = photos.iter_mut().find(|g| g.group == scanned.group) {
        existing.files.extend(scanned.files);
    } else {
        photos.push(scanned);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::{PhotoFile, PhotoGroup};
    use chrono::Utc;

    fn create_test_photo(id: &str, source: &str) -> PhotoFile {
        PhotoFile {
            id: id.to_string(),
            source: source.to_string(),
            hash: String::new(),
            width_px: 1920,
            height_px: 1080,
            area_weight: 1.0,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_merge_group_new_group() {
        let mut photos = vec![];

        let scanned = PhotoGroup {
            group: "vacation".to_string(),
            sort_key: "2024-01-15T00:00:00Z".to_string(),
            files: vec![
                create_test_photo("photo1", "/path/photo1.jpg"),
                create_test_photo("photo2", "/path/photo2.jpg"),
            ],
        };

        merge_group(&mut photos, scanned);

        assert_eq!(photos.len(), 1);
        assert_eq!(photos[0].group, "vacation");
        assert_eq!(photos[0].files.len(), 2);
    }

    #[test]
    fn test_merge_group_existing_group() {
        let mut photos = vec![PhotoGroup {
            group: "vacation".to_string(),
            sort_key: "2024-01-15T00:00:00Z".to_string(),
            files: vec![create_test_photo("photo1", "/path/photo1.jpg")],
        }];

        let scanned = PhotoGroup {
            group: "vacation".to_string(),
            sort_key: "2024-01-15T00:00:00Z".to_string(),
            files: vec![create_test_photo("photo2", "/path/photo2.jpg")],
        };

        merge_group(&mut photos, scanned);

        assert_eq!(photos.len(), 1); // Still just one group
        assert_eq!(photos[0].group, "vacation");
        assert_eq!(photos[0].files.len(), 2); // Files merged
        assert_eq!(photos[0].files[0].id, "photo1");
        assert_eq!(photos[0].files[1].id, "photo2");
    }

    #[test]
    fn test_merge_group_different_groups() {
        let mut photos = vec![PhotoGroup {
            group: "vacation".to_string(),
            sort_key: "2024-01-15T00:00:00Z".to_string(),
            files: vec![create_test_photo("photo1", "/path/photo1.jpg")],
        }];

        let scanned = PhotoGroup {
            group: "wedding".to_string(),
            sort_key: "2024-02-20T00:00:00Z".to_string(),
            files: vec![create_test_photo("photo2", "/path/photo2.jpg")],
        };

        merge_group(&mut photos, scanned);

        assert_eq!(photos.len(), 2); // Two different groups
        assert_eq!(photos[0].group, "vacation");
        assert_eq!(photos[1].group, "wedding");
        assert_eq!(photos[0].files.len(), 1);
        assert_eq!(photos[1].files.len(), 1);
    }

    #[test]
    fn test_merge_group_multiple_merges() {
        let mut photos = vec![PhotoGroup {
            group: "vacation".to_string(),
            sort_key: "2024-01-15T00:00:00Z".to_string(),
            files: vec![create_test_photo("photo1", "/path/photo1.jpg")],
        }];

        // First merge - add to existing group
        let scanned1 = PhotoGroup {
            group: "vacation".to_string(),
            sort_key: "2024-01-15T00:00:00Z".to_string(),
            files: vec![create_test_photo("photo2", "/path/photo2.jpg")],
        };
        merge_group(&mut photos, scanned1);

        // Second merge - add to existing group again
        let scanned2 = PhotoGroup {
            group: "vacation".to_string(),
            sort_key: "2024-01-15T00:00:00Z".to_string(),
            files: vec![create_test_photo("photo3", "/path/photo3.jpg")],
        };
        merge_group(&mut photos, scanned2);

        assert_eq!(photos.len(), 1);
        assert_eq!(photos[0].files.len(), 3);
        assert_eq!(photos[0].files[0].id, "photo1");
        assert_eq!(photos[0].files[1].id, "photo2");
        assert_eq!(photos[0].files[2].id, "photo3");
    }
}
