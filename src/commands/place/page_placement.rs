//! Pure chronological placement algorithm — no I/O or state mutations.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::models::{LayoutPage, PhotoFile};

#[derive(Debug, Clone)]
pub(super) struct UnplacedPhoto {
    pub(super) id: String,
    pub(super) source: String,
    pub(super) timestamp: DateTime<Utc>,
}

/// Computes (page_idx, min_timestamp, max_timestamp) for each page with photos.
/// Skips the cover page (index 0) when the cover is active.
pub(super) fn compute_page_ranges(
    layout: &[LayoutPage],
    photo_index: &HashMap<String, (PhotoFile, String)>,
    cover_active: bool,
) -> Vec<(usize, DateTime<Utc>, DateTime<Utc>)> {
    layout
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

fn min_distance(ts: DateTime<Utc>, min: DateTime<Utc>, max: DateTime<Utc>) -> u64 {
    let to_min = (ts - min).num_seconds().unsigned_abs();
    let to_max = (ts - max).num_seconds().unsigned_abs();
    to_min.min(to_max)
}

/// Finds the target page for a photo based on its timestamp.
/// `page_ranges` must already exclude the cover page if applicable.
pub(super) fn find_target_page(
    photo_ts: DateTime<Utc>,
    page_ranges: &[(usize, DateTime<Utc>, DateTime<Utc>)],
) -> usize {
    for &(idx, min_ts, max_ts) in page_ranges {
        if photo_ts >= min_ts && photo_ts <= max_ts {
            return idx;
        }
    }

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

/// Computes photo-to-page assignments chronologically.
/// Returns (page_idx, photo_id) pairs in input order.
pub(super) fn place_chronologically(
    layout: &[LayoutPage],
    photo_index: &HashMap<String, (PhotoFile, String)>,
    cover_active: bool,
    unplaced: &[&UnplacedPhoto],
) -> Vec<(usize, String)> {
    let page_ranges = compute_page_ranges(layout, photo_index, cover_active);
    unplaced
        .iter()
        .map(|photo| {
            let page_idx = find_target_page(photo.timestamp, &page_ranges);
            (page_idx, photo.id.clone())
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LayoutPage, PageMode, PhotoGroup, build_photo_index};
    use chrono::TimeZone;

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

    fn make_photo_group(files: Vec<PhotoFile>) -> Vec<PhotoGroup> {
        vec![PhotoGroup {
            group: "G".into(),
            sort_key: "2024".into(),
            files,
        }]
    }

    #[test]
    fn test_compute_page_ranges_excludes_cover_when_active() {
        let cover_ts = Utc.timestamp_opt(1000, 0).unwrap();
        let content_ts = Utc.timestamp_opt(5000, 0).unwrap();

        let photos = make_photo_group(vec![
            make_photo_file("cover.jpg", cover_ts),
            make_photo_file("content.jpg", content_ts),
        ]);
        let layout = vec![
            LayoutPage {
                photos: vec!["cover.jpg".into()],
                slots: vec![],
                mode: PageMode::Auto,
            },
            LayoutPage {
                photos: vec!["content.jpg".into()],
                slots: vec![],
                mode: PageMode::Auto,
            },
        ];
        let photo_index = build_photo_index(&photos);
        let ranges = compute_page_ranges(&layout, &photo_index, true);

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].0, 1);
    }

    #[test]
    fn test_compute_page_ranges_includes_page0_when_cover_inactive() {
        let ts0 = Utc.timestamp_opt(1000, 0).unwrap();
        let ts1 = Utc.timestamp_opt(5000, 0).unwrap();

        let photos = make_photo_group(vec![
            make_photo_file("p0.jpg", ts0),
            make_photo_file("p1.jpg", ts1),
        ]);
        let layout = vec![
            LayoutPage {
                photos: vec!["p0.jpg".into()],
                slots: vec![],
                mode: PageMode::Auto,
            },
            LayoutPage {
                photos: vec!["p1.jpg".into()],
                slots: vec![],
                mode: PageMode::Auto,
            },
        ];
        let photo_index = build_photo_index(&photos);
        let ranges = compute_page_ranges(&layout, &photo_index, false);

        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn test_find_target_page_within_range() {
        let ts_min = Utc.timestamp_opt(1000, 0).unwrap();
        let ts_max = Utc.timestamp_opt(5000, 0).unwrap();
        let page_ranges = vec![(1, ts_min, ts_max)];

        assert_eq!(
            find_target_page(Utc.timestamp_opt(3000, 0).unwrap(), &page_ranges),
            1
        );
    }

    #[test]
    fn test_find_target_page_ties_earlier_page() {
        let ts_a = Utc.timestamp_opt(1000, 0).unwrap();
        let ts_b = Utc.timestamp_opt(3000, 0).unwrap();
        let page_ranges = vec![(0, ts_a, ts_a), (2, ts_b, ts_b)];
        // midpoint ts=2000 is equidistant → earlier page wins
        let result = find_target_page(Utc.timestamp_opt(2000, 0).unwrap(), &page_ranges);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_place_chronologically_does_not_place_on_cover() {
        let cover_ts = Utc.timestamp_opt(1000, 0).unwrap();
        let content_ts = Utc.timestamp_opt(5000, 0).unwrap();
        let new_ts = Utc.timestamp_opt(1500, 0).unwrap();

        let photos = make_photo_group(vec![
            make_photo_file("cover.jpg", cover_ts),
            make_photo_file("content.jpg", content_ts),
        ]);
        let layout = vec![
            LayoutPage {
                photos: vec!["cover.jpg".into()],
                slots: vec![],
                mode: PageMode::Auto,
            },
            LayoutPage {
                photos: vec!["content.jpg".into()],
                slots: vec![],
                mode: PageMode::Auto,
            },
        ];
        let photo_index = build_photo_index(&photos);

        let new_photo = UnplacedPhoto {
            id: "new.jpg".into(),
            source: "/photos/new.jpg".into(),
            timestamp: new_ts,
        };
        let refs = vec![&new_photo];
        let assignments = place_chronologically(&layout, &photo_index, true, &refs);

        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0], (1, "new.jpg".to_string()));
    }
}
