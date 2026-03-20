//! `fotobuch place` command - Place unplaced photos into the book

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::commands::build::build_photo_index;
use crate::dto_models::{PhotoFile, ProjectState};
use crate::state_manager::StateManager;

/// Configuration for placing photos
#[derive(Debug, Clone)]
pub struct PlaceConfig {
    /// Only place photos matching these patterns (all must match)
    pub filters: Vec<String>,
    /// Place all matching photos onto this page (optional)
    pub into_page: Option<usize>,
}

/// Result of placing photos
#[derive(Debug)]
pub struct PlaceResult {
    /// Number of photos placed
    pub photos_placed: usize,
    /// Pages affected by placements (need rebuild)
    pub pages_affected: Vec<usize>,
}

/// Represents an unplaced photo with its key metadata
#[derive(Debug, Clone)]
struct UnplacedPhoto {
    id: String,
    source: String,
    timestamp: DateTime<Utc>,
}

/// Finds all photos that are in state.photos but not in state.layout
/// Returns them sorted chronologically by timestamp
fn find_unplaced(state: &ProjectState) -> Vec<UnplacedPhoto> {
    let placed_ids: HashSet<&str> = state
        .layout
        .iter()
        .flat_map(|p| p.photos.iter().map(String::as_str))
        .collect();

    let mut unplaced: Vec<UnplacedPhoto> = state
        .photos
        .iter()
        .flat_map(|g| {
            g.files.iter().map(|f| UnplacedPhoto {
                id: f.id.clone(),
                source: f.source.clone(),
                timestamp: f.timestamp,
            })
        })
        .filter(|f| !placed_ids.contains(f.id.as_str()))
        .collect();

    unplaced.sort_by_key(|f| f.timestamp);
    unplaced
}

/// Place unplaced photos into the book
///
/// # Steps
/// 1. Find unplaced photos (in photos, not in layout)
/// 2. Apply filter if provided
/// 3. If into_page: place all matching photos onto that page
/// 4. Else: sort chronologically, insert into appropriate pages based on timestamp
/// 5. Update fotobuch.yaml (layout[].photos)
/// 6. Git commit: "place: N photos"
///
/// # Arguments
/// * `project_root` - Path to the project directory
/// * `config` - Configuration for placing photos
///
/// # Returns
/// * `PlaceResult` with count of placed photos and affected pages
pub fn place(project_root: &Path, config: &PlaceConfig) -> Result<PlaceResult> {
    let mut mgr = StateManager::open(project_root)?;

    // Validation
    if mgr.state.layout.is_empty() {
        anyhow::bail!("No layout yet. Run `fotobuch build` first.");
    }
    if let Some(page) = config.into_page
        && page >= mgr.state.layout.len()
    {
        anyhow::bail!(
            "Invalid page {} (layout has {} pages, indices 0..{})",
            page,
            mgr.state.layout.len(),
            mgr.state.layout.len().saturating_sub(1),
        );
    }

    // 1. Find unplaced photos
    let unplaced = find_unplaced(&mgr.state);
    if unplaced.is_empty() {
        return Ok(PlaceResult {
            photos_placed: 0,
            pages_affected: vec![],
        });
    }

    // 2. Apply filters
    let filtered = apply_filters(&unplaced, &config.filters)?;
    if filtered.is_empty() {
        return Ok(PlaceResult {
            photos_placed: 0,
            pages_affected: vec![],
        });
    }

    // 3. Place photos
    let pages_affected = if let Some(page) = config.into_page {
        place_into_page(&mut mgr.state, &filtered, page)
    } else {
        place_chronologically(&mut mgr.state, &filtered)
    };

    let photos_placed = filtered.len();

    // 4. Commit
    let pages_str = format_page_list(&pages_affected);
    mgr.finish(&format!("place: {photos_placed} photos onto {pages_str}"))?;

    Ok(PlaceResult {
        photos_placed,
        pages_affected,
    })
}

/// Applies regex filters to unplaced photos based on their source path.
/// All filters must match (AND logic).
fn apply_filters<'a>(
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

/// Computes (page_idx, min_timestamp, max_timestamp) for each page with photos.
/// Skips the cover page (index 0) when the cover is active.
fn compute_page_ranges(
    state: &ProjectState,
    photo_index: &HashMap<String, (PhotoFile, String)>,
) -> Vec<(usize, DateTime<Utc>, DateTime<Utc>)> {
    let cover_active = state.config.book.cover.active;
    state
        .layout
        .iter()
        .enumerate()
        .filter(|(idx, _)| !cover_active || *idx != 0)
        .filter_map(|(idx, page)| {
            let timestamps: Vec<DateTime<Utc>> = page
                .photos
                .iter()
                .filter_map(|id| photo_index.get(id))
                .map(|(pf, _)| pf.timestamp)
                .collect();
            if timestamps.is_empty() {
                return None;
            }
            let min = *timestamps.iter().min().unwrap();
            let max = *timestamps.iter().max().unwrap();
            Some((idx, min, max))
        })
        .collect()
}

/// Computes minimum distance from a timestamp to a page range
fn min_distance(ts: DateTime<Utc>, min: DateTime<Utc>, max: DateTime<Utc>) -> u64 {
    let to_min = (ts - min).num_seconds().unsigned_abs();
    let to_max = (ts - max).num_seconds().unsigned_abs();
    to_min.min(to_max)
}

/// Finds the target page for a photo based on its timestamp.
/// `page_ranges` must already exclude the cover page if applicable.
/// Returns the 0-based index of the target page.
fn find_target_page(
    photo_ts: DateTime<Utc>,
    page_ranges: &[(usize, DateTime<Utc>, DateTime<Utc>)],
) -> usize {
    // Check if timestamp is within any page range
    for &(idx, min_ts, max_ts) in page_ranges {
        if photo_ts >= min_ts && photo_ts <= max_ts {
            return idx;
        }
    }

    // Find closest page by minimum distance, with tie-breaking for earlier page
    page_ranges
        .iter()
        .min_by(|a, b| {
            let dist_a = min_distance(photo_ts, a.1, a.2);
            let dist_b = min_distance(photo_ts, b.1, b.2);
            dist_a.cmp(&dist_b).then(a.0.cmp(&b.0))
        })
        .map(|&(idx, _, _)| idx)
        .unwrap_or(0)
}

/// Places photos chronologically onto appropriate pages
/// Returns affected page indices (0-based, sorted, deduplicated)
fn place_chronologically(state: &mut ProjectState, photos: &[&UnplacedPhoto]) -> Vec<usize> {
    let photo_index = build_photo_index(&state.photos);
    let page_ranges = compute_page_ranges(state, &photo_index);

    let mut affected: HashSet<usize> = HashSet::new();

    for photo in photos {
        let page_idx = find_target_page(photo.timestamp, &page_ranges);
        state.layout[page_idx].photos.push(photo.id.clone());
        affected.insert(page_idx);
    }

    let mut result: Vec<usize> = affected.into_iter().collect();
    result.sort();
    result
}

/// Places all photos onto a specific page
/// Returns affected page index (0-based, as single-element vector)
fn place_into_page(
    state: &mut ProjectState,
    photos: &[&UnplacedPhoto],
    page_idx: usize,
) -> Vec<usize> {
    for photo in photos {
        state.layout[page_idx].photos.push(photo.id.clone());
    }
    vec![page_idx]
}

/// Formats page list for commit message: "page 5" or "pages 2, 5, 8"
fn format_page_list(pages: &[usize]) -> String {
    if pages.len() == 1 {
        format!("page {}", pages[0])
    } else {
        let list: Vec<String> = pages.iter().map(|p| p.to_string()).collect();
        format!("pages {}", list.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::{LayoutPage, PhotoGroup, ProjectState};
    use chrono::TimeZone;

    fn make_unplaced(id: &str, source: &str, ts: DateTime<Utc>) -> UnplacedPhoto {
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
                page: 1,
                photos: vec!["a.jpg".to_string()],
                slots: vec![],
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
        // Both patterns must match
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

    #[test]
    fn test_format_page_list_single() {
        assert_eq!(format_page_list(&[5]), "page 5");
    }

    #[test]
    fn test_format_page_list_multiple() {
        assert_eq!(format_page_list(&[2, 5, 8]), "pages 2, 5, 8");
    }

    fn make_photo_file(id: &str, ts: DateTime<Utc>) -> PhotoFile {
        PhotoFile {
            id: id.to_string(),
            source: format!("/photos/{id}"),
            width_px: 1920,
            height_px: 1080,
            area_weight: 1.0,
            timestamp: ts,
            hash: id.to_string(),
        }
    }

    /// Build a minimal ProjectState with an optional active cover.
    fn make_state_with_cover(cover_active: bool, layout: Vec<LayoutPage>, photos: Vec<PhotoGroup>) -> ProjectState {
        use crate::dto_models::{BookConfig, CoverConfig, ProjectConfig};
        ProjectState {
            config: ProjectConfig {
                book: BookConfig {
                    title: "Test".into(),
                    page_width_mm: 210.0,
                    page_height_mm: 297.0,
                    bleed_mm: 3.0,
                    margin_mm: 10.0,
                    gap_mm: 5.0,
                    bleed_threshold_mm: 3.0,
                    dpi: 300.0,
                    cover: CoverConfig { active: cover_active, ..Default::default() },
                },
                ..Default::default()
            },
            photos,
            layout,
        }
    }

    #[test]
    fn test_compute_page_ranges_excludes_cover_when_active() {
        // Page 0 = cover (ts=1000), page 1 = content (ts=5000)
        let cover_ts = Utc.timestamp_opt(1000, 0).unwrap();
        let content_ts = Utc.timestamp_opt(5000, 0).unwrap();

        let photos = vec![PhotoGroup {
            group: "G".into(),
            sort_key: "2024".into(),
            files: vec![
                make_photo_file("cover.jpg", cover_ts),
                make_photo_file("content.jpg", content_ts),
            ],
        }];
        let layout = vec![
            LayoutPage { page: 0, photos: vec!["cover.jpg".into()], slots: vec![] },
            LayoutPage { page: 1, photos: vec!["content.jpg".into()], slots: vec![] },
        ];
        let state = make_state_with_cover(true, layout, photos);
        let photo_index = crate::commands::build::build_photo_index(&state.photos);
        let ranges = compute_page_ranges(&state, &photo_index);

        // Cover (page 0) must be absent; only page 1 should appear
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].0, 1);
    }

    #[test]
    fn test_compute_page_ranges_includes_page0_when_cover_inactive() {
        let ts0 = Utc.timestamp_opt(1000, 0).unwrap();
        let ts1 = Utc.timestamp_opt(5000, 0).unwrap();

        let photos = vec![PhotoGroup {
            group: "G".into(),
            sort_key: "2024".into(),
            files: vec![
                make_photo_file("p0.jpg", ts0),
                make_photo_file("p1.jpg", ts1),
            ],
        }];
        let layout = vec![
            LayoutPage { page: 0, photos: vec!["p0.jpg".into()], slots: vec![] },
            LayoutPage { page: 1, photos: vec!["p1.jpg".into()], slots: vec![] },
        ];
        let state = make_state_with_cover(false, layout, photos);
        let photo_index = crate::commands::build::build_photo_index(&state.photos);
        let ranges = compute_page_ranges(&state, &photo_index);

        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn test_place_chronologically_does_not_place_on_cover() {
        // Cover (page 0) has a photo at ts=1000; content page (page 1) has a photo at ts=5000.
        // A new photo at ts=1500 should go to page 1 (closest content page), not cover.
        let cover_ts = Utc.timestamp_opt(1000, 0).unwrap();
        let content_ts = Utc.timestamp_opt(5000, 0).unwrap();
        let new_ts = Utc.timestamp_opt(1500, 0).unwrap();

        let photos = vec![PhotoGroup {
            group: "G".into(),
            sort_key: "2024".into(),
            files: vec![
                make_photo_file("cover.jpg", cover_ts),
                make_photo_file("content.jpg", content_ts),
                make_photo_file("new.jpg", new_ts),
            ],
        }];
        let layout = vec![
            LayoutPage { page: 0, photos: vec!["cover.jpg".into()], slots: vec![] },
            LayoutPage { page: 1, photos: vec!["content.jpg".into()], slots: vec![] },
        ];
        let mut state = make_state_with_cover(true, layout, photos);

        let new_photo = UnplacedPhoto {
            id: "new.jpg".into(),
            source: "/photos/new.jpg".into(),
            timestamp: new_ts,
        };
        let refs: Vec<&UnplacedPhoto> = vec![&new_photo];
        let affected = place_chronologically(&mut state, &refs);

        // Must NOT land on cover (page 0)
        assert!(!state.layout[0].photos.contains(&"new.jpg".to_string()));
        assert!(state.layout[1].photos.contains(&"new.jpg".to_string()));
        assert_eq!(affected, vec![1]);
    }
}
