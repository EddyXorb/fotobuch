//! Selecting which unplaced photos to place: discovery + regex filtering.

use anyhow::{Context, Result};
use regex::Regex;

use crate::models::ProjectState;

use super::page_placement::UnplacedPhoto;

/// All photos that exist in `photos` but are not placed in any layout page,
/// sorted chronologically by timestamp.
pub(super) fn find_unplaced(state: &ProjectState) -> Vec<UnplacedPhoto> {
    let mut unplaced: Vec<UnplacedPhoto> = state
        .unplaced_photo_files()
        .map(|f| UnplacedPhoto {
            id: f.id.clone(),
            source: f.source.clone(),
            timestamp: f.timestamp,
        })
        .collect();
    unplaced.sort_by_key(|f| f.timestamp);
    unplaced
}

/// Applies regex filters to unplaced photos based on their source path.
/// All filters must match (AND logic).
pub(super) fn apply_filters<'a>(
    photos: &'a [UnplacedPhoto],
    patterns: &[String],
) -> Result<Vec<&'a UnplacedPhoto>> {
    if patterns.is_empty() {
        return Ok(photos.iter().collect());
    }

    let regexes: Result<Vec<Regex>> = patterns
        .iter()
        .map(|pat| Regex::new(pat).context(format!("Invalid filter pattern: {pat}")))
        .collect();
    let regexes = regexes?;

    Ok(photos
        .iter()
        .filter(|p| regexes.iter().all(|re| re.is_match(&p.source)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LayoutPage, PageMode, PhotoFile, PhotoGroup, ProjectState};
    use chrono::{TimeZone, Utc};

    fn make_unplaced(id: &str, source: &str, ts: chrono::DateTime<Utc>) -> UnplacedPhoto {
        UnplacedPhoto {
            id: id.to_string(),
            source: source.to_string(),
            timestamp: ts,
        }
    }

    #[test]
    fn test_find_unplaced_finds_missing_photos() {
        let photo1 = PhotoFile {
            id: "a.jpg".to_string(),
            source: "/path/a.jpg".to_string(),
            width_px: 1920,
            height_px: 1080,
            area_weight: 1.0,
            timestamp: Utc.timestamp_opt(1000, 0).unwrap(),
            hash: "abc".to_string(),
        };
        let photo2 = PhotoFile {
            id: "b.jpg".to_string(),
            source: "/path/b.jpg".to_string(),
            width_px: 1920,
            height_px: 1080,
            area_weight: 1.0,
            timestamp: Utc.timestamp_opt(2000, 0).unwrap(),
            hash: "def".to_string(),
        };

        let state = ProjectState {
            config: Default::default(),
            photos: vec![PhotoGroup {
                group: "Test".to_string(),
                sort_key: "2024-01-01".to_string(),
                files: vec![photo1, photo2],
            }],
            layout: vec![LayoutPage {
                photos: vec!["a.jpg".to_string()],
                slots: vec![],
                mode: PageMode::Auto,
            }],
        };

        let unplaced = find_unplaced(&state);
        assert_eq!(unplaced.len(), 1);
        assert_eq!(unplaced[0].id, "b.jpg");
    }

    #[test]
    fn test_apply_filters_no_patterns() {
        let photos = vec![make_unplaced(
            "a.jpg",
            "/path/to/a.jpg",
            Utc.timestamp_opt(1000, 0).unwrap(),
        )];
        let filtered = apply_filters(&photos, &[]).unwrap();
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_apply_filters_single_pattern() {
        let photos = vec![
            make_unplaced(
                "a.jpg",
                "/path/vacation/a.jpg",
                Utc.timestamp_opt(1000, 0).unwrap(),
            ),
            make_unplaced(
                "b.jpg",
                "/path/work/b.jpg",
                Utc.timestamp_opt(2000, 0).unwrap(),
            ),
        ];
        let filtered = apply_filters(&photos, &["vacation".to_string()]).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "a.jpg");
    }

    #[test]
    fn test_apply_filters_multiple_patterns_and_logic() {
        let photos = vec![
            make_unplaced(
                "a.jpg",
                "/path/vacation/2024/a.jpg",
                Utc.timestamp_opt(1000, 0).unwrap(),
            ),
            make_unplaced(
                "b.jpg",
                "/path/vacation/2023/b.jpg",
                Utc.timestamp_opt(2000, 0).unwrap(),
            ),
            make_unplaced(
                "c.jpg",
                "/path/work/2024/c.jpg",
                Utc.timestamp_opt(3000, 0).unwrap(),
            ),
        ];
        let filtered =
            apply_filters(&photos, &["vacation".to_string(), "2024".to_string()]).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "a.jpg");
    }

    #[test]
    fn test_apply_filters_invalid_regex() {
        let photos = vec![];
        let result = apply_filters(&photos, &["[invalid".to_string()]);
        assert!(result.is_err());
    }
}
