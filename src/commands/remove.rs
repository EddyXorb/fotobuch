//! `fotobuch remove` command - Remove photos or groups from the project

use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;

use crate::dto_models::{LayoutPage, PhotoGroup, ProjectState};
use crate::state_manager::StateManager;

/// Configuration for removing photos
#[derive(Debug, Clone)]
pub struct RemoveConfig {
    /// Photo paths, group names, or glob patterns
    pub patterns: Vec<String>,
    /// Only remove from layout, keep in photos (makes them unplaced)
    pub keep_files: bool,
    /// Remove all photos not placed in any layout page
    pub unplaced: bool,
}

/// Result of removing photos
#[derive(Debug)]
pub struct RemoveResult {
    /// Number of photos removed from photos section
    pub photos_removed: usize,
    /// Number of placements removed from layout
    pub placements_removed: usize,
    /// Groups that were completely removed
    pub groups_removed: Vec<String>,
    /// Pages affected by removals (need rebuild)
    pub pages_affected: Vec<usize>,
}

/// Matches photos by group name or regex pattern on source path
struct MatchResult {
    matched_ids: HashSet<String>,
    matched_groups: Vec<String>,
}

/// Sammelt alle Photo-IDs die mindestens einem Pattern entsprechen.
/// Patterns können exakte Gruppennamen oder Regex-Patterns sein.
fn match_photos(state: &ProjectState, patterns: &[String]) -> Result<MatchResult> {
    let mut matched_ids: HashSet<String> = HashSet::new();
    let mut matched_groups: Vec<String> = Vec::new();

    for pattern in patterns {
        // 1. Exakter Gruppenname?
        if let Some(group) = state.photos.iter().find(|g| g.group == *pattern) {
            for file in &group.files {
                matched_ids.insert(file.id.clone());
            }
            matched_groups.push(group.group.clone());
            continue;
        }

        // 2. Regex auf photo.source
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

/// Result of removing from layout
struct LayoutRemoveResult {
    placements_removed: usize,
    pages_affected: Vec<usize>, // 1-basiert
}

/// Entfernt gematchte Fotos aus allen Layout-Seiten.
/// Photos und Slots sind index-gekoppelt — beide werden parallel gefiltert.
fn remove_from_layout(
    layout: &mut [LayoutPage],
    matched_ids: &HashSet<String>,
) -> LayoutRemoveResult {
    let mut placements_removed = 0;
    let mut pages_affected = Vec::new();

    for page in layout.iter_mut() {
        let before = page.photos.len();

        // Photos und Slots parallel filtern (index-gekoppelt)
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
            // Slots vorhanden und index-gekoppelt
            page.slots
                .iter()
                .zip(&keep)
                .filter(|&(_, k)| *k)
                .map(|(slot, _)| slot.clone())
                .collect()
        } else {
            // Slots leer oder inkonsistent — leeren
            vec![]
        };

        let removed = before - new_photos.len();
        if removed > 0 {
            pages_affected.push(page.page);
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

/// Entfernt Seiten ohne Fotos aus dem Layout.
fn remove_empty_pages(layout: &mut Vec<LayoutPage>) {
    layout.retain(|p| !p.photos.is_empty());
}

/// Nummeriert alle LayoutPage.page Felder sequenziell (1-basiert).
fn renumber_pages(layout: &mut [LayoutPage]) {
    for (i, page) in layout.iter_mut().enumerate() {
        page.page = i + 1;
    }
}

/// Entfernt gematchte Fotos aus state.photos.
/// Leere Gruppen werden komplett entfernt.
/// Gibt die Anzahl entfernter Fotos zurück.
fn remove_from_photos(
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

    // Leere Gruppen entfernen
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

/// Collects IDs of all photos not referenced in any layout page.
fn collect_unplaced_ids(state: &ProjectState) -> HashSet<String> {
    let placed: HashSet<&str> = state
        .layout
        .iter()
        .flat_map(|p| p.photos.iter())
        .map(|id| id.as_str())
        .collect();

    state
        .photos
        .iter()
        .flat_map(|g| g.files.iter())
        .filter(|f| !placed.contains(f.id.as_str()))
        .map(|f| f.id.clone())
        .collect()
}

/// Remove photos or groups from the project
///
/// # Steps
/// 1. Match patterns (group names, regex on source paths) OR collect unplaced IDs
/// 2. Remove from layout (always, noop for --unplaced since they aren't placed)
/// 3. Remove empty pages + renumber
/// 4. Remove from photos (if not keep_files)
/// 5. Update fotobuch.yaml
/// 6. Git commit
///
/// # Arguments
/// * `project_root` - Path to the project directory
/// * `config` - Configuration for removing photos
///
/// # Returns
/// * `RemoveResult` with summary of removed photos and affected pages
pub fn remove(project_root: &Path, config: &RemoveConfig) -> Result<RemoveResult> {
    let mut mgr = StateManager::open(project_root)?;

    // 1. Determine which IDs to act on
    let (matched_ids, matched_groups) = if config.unplaced {
        (collect_unplaced_ids(&mgr.state), vec![])
    } else {
        let matches = match_photos(&mgr.state, &config.patterns)?;
        (matches.matched_ids, matches.matched_groups)
    };

    if matched_ids.is_empty() {
        return Ok(RemoveResult {
            photos_removed: 0,
            placements_removed: 0,
            groups_removed: vec![],
            pages_affected: vec![],
        });
    }

    // 2. Aus Layout entfernen (immer, auch bei --keep-files)
    let layout_result = remove_from_layout(&mut mgr.state.layout, &matched_ids);

    // 3. Leere Seiten entfernen + renumbern
    remove_empty_pages(&mut mgr.state.layout);
    renumber_pages(&mut mgr.state.layout);

    // 4. Aus Photos entfernen (nur ohne --keep-files)
    let mut groups_removed = matched_groups;
    let photos_removed = if config.keep_files {
        0
    } else {
        remove_from_photos(
            &mut mgr.state.photos,
            &matched_ids,
            &mut groups_removed,
        )
    };

    // 5. Speichern + Git commit
    let commit_msg = if config.unplaced {
        format!("remove: {} unplaced photos", photos_removed)
    } else if config.keep_files {
        format!(
            "remove: {} placements from layout (photos kept)",
            layout_result.placements_removed
        )
    } else {
        format!("remove: {} photos", photos_removed)
    };
    mgr.finish(&commit_msg)?;

    Ok(RemoveResult {
        photos_removed,
        placements_removed: layout_result.placements_removed,
        groups_removed,
        pages_affected: layout_result.pages_affected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::{LayoutPage, PhotoFile, Slot};
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
            page: 1,
            photos: vec!["a.jpg".to_string(), "b.jpg".to_string()],
            slots: vec![slot1.clone(), slot2.clone()],
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
    fn test_remove_empty_pages() {
        let mut layout = vec![
            LayoutPage {
                page: 1,
                photos: vec![],
                slots: vec![],
            },
            LayoutPage {
                page: 2,
                photos: vec!["a.jpg".to_string()],
                slots: vec![],
            },
            LayoutPage {
                page: 3,
                photos: vec![],
                slots: vec![],
            },
        ];

        remove_empty_pages(&mut layout);
        assert_eq!(layout.len(), 1);
        assert_eq!(layout[0].page, 2);
    }

    #[test]
    fn test_renumber_pages() {
        let mut layout = vec![
            LayoutPage {
                page: 5,
                photos: vec!["a.jpg".to_string()],
                slots: vec![],
            },
            LayoutPage {
                page: 7,
                photos: vec!["b.jpg".to_string()],
                slots: vec![],
            },
        ];

        renumber_pages(&mut layout);
        assert_eq!(layout[0].page, 1);
        assert_eq!(layout[1].page, 2);
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
                page: 1,
                photos: vec!["a.jpg".to_string()],
                slots: vec![],
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
                page: 1,
                photos: vec!["a.jpg".to_string()],
                slots: vec![],
            }],
        };

        let unplaced = collect_unplaced_ids(&state);
        assert!(unplaced.is_empty());
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
}
