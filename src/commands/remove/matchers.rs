//! Pure matching and removal operations — no I/O or state mutations.

use std::collections::HashSet;

use anyhow::{Context, Result};
use regex::Regex;

use crate::models::{LayoutPage, PhotoGroup, ProjectState};

pub(super) struct MatchResult {
    pub(super) matched_ids: HashSet<String>,
    pub(super) matched_groups: Vec<String>,
}

/// Sammelt alle Photo-IDs die mindestens einem Pattern entsprechen.
/// Patterns können exakte Gruppennamen oder Regex-Patterns sein.
pub(super) fn match_photos(state: &ProjectState, patterns: &[String]) -> Result<MatchResult> {
    let mut matched_ids: HashSet<String> = HashSet::new();
    let mut matched_groups: Vec<String> = Vec::new();

    for pattern in patterns {
        if let Some(group) = state.photos.iter().find(|g| g.group == *pattern) {
            for file in &group.files {
                matched_ids.insert(file.id.clone());
            }
            matched_groups.push(group.group.clone());
            continue;
        }

        let re = Regex::new(pattern).context(format!("Invalid pattern: {pattern}"))?;
        for group in &state.photos {
            for file in &group.files {
                if re.is_match(&file.source) {
                    matched_ids.insert(file.id.clone());
                }
            }
        }
    }

    Ok(MatchResult {
        matched_ids,
        matched_groups,
    })
}

pub(super) struct LayoutRemoveResult {
    pub(super) placements_removed: usize,
    pub(super) pages_affected: Vec<usize>,
}

/// Entfernt gematchte Fotos aus allen Layout-Seiten.
/// Photos und Slots sind index-gekoppelt — beide werden parallel gefiltert.
pub(super) fn remove_from_layout(
    layout: &mut [LayoutPage],
    matched_ids: &HashSet<String>,
) -> LayoutRemoveResult {
    let mut placements_removed = 0;
    let mut pages_affected = Vec::new();

    for (page_idx, page) in layout.iter_mut().enumerate() {
        let before = page.photos.len();

        let keep: Vec<bool> = page
            .photos
            .iter()
            .map(|id| !matched_ids.contains(id))
            .collect();

        let new_photos: Vec<String> = page
            .photos
            .iter()
            .zip(&keep)
            .filter(|&(_, k)| *k)
            .map(|(id, _)| id.clone())
            .collect();

        let new_slots = if page.slots.len() == page.photos.len() {
            page.slots
                .iter()
                .zip(&keep)
                .filter(|&(_, k)| *k)
                .map(|(slot, _)| slot.clone())
                .collect()
        } else {
            vec![]
        };

        let removed = before - new_photos.len();
        if removed > 0 {
            pages_affected.push(page_idx);
            placements_removed += removed;
        }

        page.photos = new_photos;
        page.slots = new_slots;
    }

    LayoutRemoveResult {
        placements_removed,
        pages_affected,
    }
}

/// Entfernt gematchte Fotos aus state.photos.
/// Leere Gruppen werden komplett entfernt.
pub(super) fn remove_from_photos(
    photos: &mut Vec<PhotoGroup>,
    matched_ids: &HashSet<String>,
    groups_removed: &mut Vec<String>,
) -> usize {
    let mut total_removed = 0;

    for group in photos.iter_mut() {
        let before = group.files.len();
        group.files.retain(|f| !matched_ids.contains(&f.id));
        total_removed += before - group.files.len();
    }

    let empty_groups: Vec<String> = photos
        .iter()
        .filter(|g| g.files.is_empty())
        .map(|g| g.group.clone())
        .collect();

    for g in &empty_groups {
        if !groups_removed.contains(g) {
            groups_removed.push(g.clone());
        }
    }

    photos.retain(|g| !g.files.is_empty());
    total_removed
}

pub(super) fn collect_unplaced_ids(state: &ProjectState) -> HashSet<String> {
    state.unplaced_photo_files().map(|f| f.id.clone()).collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LayoutPage, PageMode, PhotoFile, PhotoGroup, ProjectState, Slot};
    use chrono::Utc;

    fn make_photo(id: &str, source: &str) -> PhotoFile {
        PhotoFile {
            id: id.to_string(),
            source: source.to_string(),
            width_px: 1920,
            height_px: 1080,
            area_weight: 1.0,
            timestamp: Utc::now(),
            hash: "test".to_string(),
        }
    }

    #[test]
    fn test_match_photos_by_group_name() {
        let state = ProjectState {
            config: Default::default(),
            photos: vec![PhotoGroup {
                group: "Vacation".to_string(),
                sort_key: "2024-01-01".to_string(),
                files: vec![
                    make_photo("v1.jpg", "/photos/v1.jpg"),
                    make_photo("v2.jpg", "/photos/v2.jpg"),
                ],
            }],
            layout: vec![],
        };

        let result = match_photos(&state, &["Vacation".to_string()]).unwrap();
        assert_eq!(result.matched_ids.len(), 2);
        assert_eq!(result.matched_groups.len(), 1);
        assert!(result.matched_ids.contains("v1.jpg"));
        assert!(result.matched_ids.contains("v2.jpg"));
    }

    #[test]
    fn test_match_photos_by_regex() {
        let state = ProjectState {
            config: Default::default(),
            photos: vec![PhotoGroup {
                group: "Test".to_string(),
                sort_key: "2024-01-01".to_string(),
                files: vec![
                    make_photo("a.jpg", "/path/vacation/a.jpg"),
                    make_photo("b.jpg", "/path/work/b.jpg"),
                ],
            }],
            layout: vec![],
        };

        let result = match_photos(&state, &["vacation".to_string()]).unwrap();
        assert_eq!(result.matched_ids.len(), 1);
        assert!(result.matched_ids.contains("a.jpg"));
    }

    #[test]
    fn test_match_photos_invalid_regex() {
        let state = ProjectState::default();
        let result = match_photos(&state, &["[invalid".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_from_layout_basic() {
        let slot1 = Slot {
            x_mm: 10.0,
            y_mm: 10.0,
            width_mm: 100.0,
            height_mm: 100.0,
        };
        let slot2 = Slot {
            x_mm: 120.0,
            y_mm: 10.0,
            width_mm: 100.0,
            height_mm: 100.0,
        };

        let mut layout = vec![LayoutPage {
            photos: vec!["a.jpg".to_string(), "b.jpg".to_string()],
            slots: vec![slot1.clone(), slot2.clone()],
            mode: PageMode::Auto,
        }];

        let mut matched = HashSet::new();
        matched.insert("a.jpg".to_string());

        let result = remove_from_layout(&mut layout, &matched);
        assert_eq!(result.placements_removed, 1);
        assert_eq!(layout[0].photos.len(), 1);
        assert_eq!(layout[0].photos[0], "b.jpg");
        assert_eq!(layout[0].slots.len(), 1);
        assert_eq!(layout[0].slots[0], slot2);
    }

    #[test]
    fn test_remove_from_photos() {
        let mut photos = vec![PhotoGroup {
            group: "Group1".to_string(),
            sort_key: "2024-01-01".to_string(),
            files: vec![
                make_photo("a.jpg", "/path/a.jpg"),
                make_photo("b.jpg", "/path/b.jpg"),
            ],
        }];

        let mut matched = HashSet::new();
        matched.insert("a.jpg".to_string());

        let mut groups_removed = vec![];
        let removed = remove_from_photos(&mut photos, &matched, &mut groups_removed);

        assert_eq!(removed, 1);
        assert_eq!(photos[0].files.len(), 1);
        assert_eq!(photos[0].files[0].id, "b.jpg");
    }

    #[test]
    fn test_remove_from_photos_empty_group() {
        let mut photos = vec![PhotoGroup {
            group: "Group1".to_string(),
            sort_key: "2024-01-01".to_string(),
            files: vec![make_photo("a.jpg", "/path/a.jpg")],
        }];

        let mut matched = HashSet::new();
        matched.insert("a.jpg".to_string());

        let mut groups_removed = vec![];
        let removed = remove_from_photos(&mut photos, &matched, &mut groups_removed);

        assert_eq!(removed, 1);
        assert!(photos.is_empty());
        assert!(groups_removed.contains(&"Group1".to_string()));
    }

    #[test]
    fn test_collect_unplaced_ids_all_unplaced() {
        let state = ProjectState {
            config: Default::default(),
            photos: vec![PhotoGroup {
                group: "Group1".to_string(),
                sort_key: "2024-01-01".to_string(),
                files: vec![
                    make_photo("a.jpg", "/path/a.jpg"),
                    make_photo("b.jpg", "/path/b.jpg"),
                ],
            }],
            layout: vec![],
        };

        let unplaced = collect_unplaced_ids(&state);
        assert_eq!(unplaced.len(), 2);
        assert!(unplaced.contains("a.jpg"));
        assert!(unplaced.contains("b.jpg"));
    }

    #[test]
    fn test_collect_unplaced_ids_some_placed() {
        let state = ProjectState {
            config: Default::default(),
            photos: vec![PhotoGroup {
                group: "Group1".to_string(),
                sort_key: "2024-01-01".to_string(),
                files: vec![
                    make_photo("a.jpg", "/path/a.jpg"),
                    make_photo("b.jpg", "/path/b.jpg"),
                ],
            }],
            layout: vec![LayoutPage {
                photos: vec!["a.jpg".to_string()],
                slots: vec![],
                mode: PageMode::Auto,
            }],
        };

        let unplaced = collect_unplaced_ids(&state);
        assert_eq!(unplaced.len(), 1);
        assert!(unplaced.contains("b.jpg"));
        assert!(!unplaced.contains("a.jpg"));
    }

    #[test]
    fn test_collect_unplaced_ids_all_placed() {
        let state = ProjectState {
            config: Default::default(),
            photos: vec![PhotoGroup {
                group: "Group1".to_string(),
                sort_key: "2024-01-01".to_string(),
                files: vec![make_photo("a.jpg", "/path/a.jpg")],
            }],
            layout: vec![LayoutPage {
                photos: vec!["a.jpg".to_string()],
                slots: vec![],
                mode: PageMode::Auto,
            }],
        };

        let unplaced = collect_unplaced_ids(&state);
        assert!(unplaced.is_empty());
    }
}
