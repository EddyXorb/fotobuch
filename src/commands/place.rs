//! `fotobuch place` command - Place unplaced photos into the book

mod page_placement;
use page_placement::UnplacedPhoto;

use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;

use crate::commands::CommandOutput;
use crate::commands::page::format_pages_list;
use crate::models::{LayoutPage, PageMode, ProjectState, build_photo_index};
use crate::state_manager::{ReadOnlyState, StateManager, WriteLayoutState};

/// Target destination for placing photos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceDst {
    /// Place chronologically across existing pages (default).
    Auto,
    /// Place onto an existing page (0-based index).
    Page(usize),
    /// Create a new page at this position and place photos there.
    NewPageAt(usize),
}

/// Configuration for placing photos
#[derive(Debug, Clone)]
pub struct PlaceConfig {
    /// Only place photos matching these patterns (all must match)
    pub filters: Vec<String>,
    /// Restrict to these photo IDs (empty = no restriction)
    pub ids: Vec<String>,
    /// Where to place the photos
    pub dst: PlaceDst,
}

/// Result of placing photos
#[derive(Debug)]
pub struct PlaceResult {
    /// Number of photos placed
    pub photos_placed: usize,
    /// Pages affected by placements (need rebuild)
    pub pages_affected: Vec<usize>,
    /// Pages newly inserted (0-based positions after insertion)
    pub pages_inserted: Vec<usize>,
}

fn find_unplaced(state: &ProjectState) -> Vec<UnplacedPhoto> {
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
pub fn place(project_root: &Path, config: &PlaceConfig) -> Result<CommandOutput<PlaceResult>> {
    let mut mgr = StateManager::open(project_root)?;

    // Validation
    if mgr.state().layout.is_empty() && !matches!(config.dst, PlaceDst::NewPageAt(_)) {
        anyhow::bail!("No layout yet. Run `fotobuch build` first.");
    }
    match config.dst {
        PlaceDst::Page(page) if page >= mgr.state().layout.len() => {
            anyhow::bail!(
                "Invalid page {} (layout has {} pages, indices 0..{})",
                page,
                mgr.state().layout.len(),
                mgr.state().layout.len().saturating_sub(1),
            );
        }
        PlaceDst::NewPageAt(pos) if pos > mgr.state().layout.len() => {
            anyhow::bail!(
                "Invalid new page position {} (layout has {} pages, valid range 0..={})",
                pos,
                mgr.state().layout.len(),
                mgr.state().layout.len(),
            );
        }
        _ => {}
    }

    // 1. Find unplaced photos
    let unplaced = find_unplaced(mgr.state());
    if unplaced.is_empty() {
        let changed_state = mgr.finish("")?;
        return Ok(CommandOutput {
            result: PlaceResult {
                photos_placed: 0,
                pages_affected: vec![],
                pages_inserted: vec![],
            },
            changed_state,
        });
    }

    // 2. Apply filters
    let after_regex = apply_filters(&unplaced, &config.filters)?;
    let filtered: Vec<_> = if config.ids.is_empty() {
        after_regex
    } else {
        after_regex
            .into_iter()
            .filter(|p| config.ids.contains(&p.id))
            .collect()
    };
    if filtered.is_empty() {
        let changed_state = mgr.finish("")?;
        return Ok(CommandOutput {
            result: PlaceResult {
                photos_placed: 0,
                pages_affected: vec![],
                pages_inserted: vec![],
            },
            changed_state,
        });
    }

    // 3. Place photos (read phase before write)
    let (pages_affected, pages_inserted) = {
        let mut wls: WriteLayoutState<'_> = mgr.get_write_layout_state();
        match config.dst {
            PlaceDst::NewPageAt(pos) => place_into_new_page(wls.layout_mut(), &filtered, pos),
            PlaceDst::Page(page) => (place_into_page(wls.layout_mut(), &filtered, page), vec![]),
            PlaceDst::Auto => {
                let photo_index = build_photo_index(wls.photos());
                let cover_active = wls.config().book.cover.active;
                let assignments = page_placement::place_chronologically(
                    wls.layout(),
                    &photo_index,
                    cover_active,
                    &filtered,
                );
                let mut affected = HashSet::new();
                for (page_idx, photo_id) in assignments {
                    wls.layout_mut()[page_idx].photos.push(photo_id);
                    affected.insert(page_idx);
                }
                let mut result: Vec<usize> = affected.into_iter().collect();
                result.sort();
                (result, vec![])
            }
        }
    };

    let photos_placed = filtered.len();

    // 4. Commit
    let pages_u32: Vec<u32> = pages_affected.iter().map(|&p| p as u32).collect();
    let pages_str = format_pages_list(&pages_u32);
    let changed_state = mgr.finish(&format!("place: {photos_placed} photos onto {pages_str}"))?;

    Ok(CommandOutput {
        result: PlaceResult {
            photos_placed,
            pages_affected,
            pages_inserted,
        },
        changed_state,
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

/// Places all photos onto a specific page
/// Returns affected page index (0-based, as single-element vector)
fn place_into_page(
    layout: &mut [LayoutPage],
    photos: &[&UnplacedPhoto],
    page_idx: usize,
) -> Vec<usize> {
    for photo in photos {
        layout[page_idx].photos.push(photo.id.clone());
    }
    vec![page_idx]
}

/// Creates a new page at the given position and places all photos there.
/// Returns (affected pages, inserted pages).
fn place_into_new_page(
    layout: &mut Vec<LayoutPage>,
    photos: &[&UnplacedPhoto],
    position: usize,
) -> (Vec<usize>, Vec<usize>) {
    let photo_ids: Vec<String> = photos.iter().map(|p| p.id.clone()).collect();
    layout.insert(
        position,
        LayoutPage {
            photos: photo_ids,
            slots: vec![],
            mode: PageMode::Auto,
        },
    );
    (vec![position], vec![position])
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
