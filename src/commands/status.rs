//! `fotobuch status` command - Show project status without modifying anything

use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

use crate::commands::build::build_photo_index;
use crate::dto_models::ProjectState;
use crate::state_manager::StateManager;

/// Configuration for status command
#[derive(Debug, Clone)]
pub struct StatusConfig {
    /// Show detail for a specific page (1-based)
    pub page: Option<usize>,
}

/// Overall project state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectState_ {
    /// No layout exists yet
    Empty,
    /// Layout exists, nothing changed since last build
    Clean,
    /// Layout exists, changed since last build
    Modified,
}

/// Information about a single photo on a page
#[derive(Debug, Clone)]
pub struct SlotInfo {
    /// Photo ID
    pub photo_id: String,
    /// Aspect ratio (width/height)
    pub ratio: f64,
    /// Swap group (A, B, C, ...) for compatible ratios
    pub swap_group: char,
    /// Slot dimensions: (x_mm, y_mm, width_mm, height_mm)
    pub slot_mm: (f64, f64, f64, f64),
}

/// Detailed information about a single page
#[derive(Debug, Clone)]
pub struct PageDetail {
    /// Page number (1-based)
    pub page: usize,
    /// Number of photos on this page
    pub photo_count: usize,
    /// Whether this page was modified since last build
    pub modified: bool,
    /// Information about each photo slot
    pub slots: Vec<SlotInfo>,
}

/// Overall project status report
#[derive(Debug, Clone)]
pub struct StatusReport {
    /// Project name
    pub project_name: String,
    /// Overall state (empty/clean/modified)
    pub state: ProjectState_,
    /// Total number of photos
    pub total_photos: usize,
    /// Number of groups
    pub group_count: usize,
    /// Number of unplaced photos
    pub unplaced: usize,
    /// Number of pages in layout
    pub page_count: usize,
    /// Average photos per page
    pub avg_photos_per_page: f64,
    /// 1-based page numbers that were modified since last build
    pub page_changes: Vec<usize>,
    /// Detailed info for a specific page (if requested)
    pub detail: Option<PageDetail>,
    /// Warnings (e.g., orphaned placements)
    pub warnings: Vec<String>,
}

/// Count photos not placed in layout
fn count_unplaced(state: &ProjectState) -> usize {
    let placed_ids: HashSet<&str> = state
        .layout
        .iter()
        .flat_map(|p| p.photos.iter().map(String::as_str))
        .collect();
    state
        .photos
        .iter()
        .flat_map(|g| &g.files)
        .filter(|f| !placed_ids.contains(f.id.as_str()))
        .count()
}

/// Check consistency between photos and layout
fn check_consistency(state: &ProjectState) -> Vec<String> {
    let photo_index = build_photo_index(&state.photos);
    let placed_ids: HashSet<&str> = state
        .layout
        .iter()
        .flat_map(|p| p.photos.iter().map(String::as_str))
        .collect();
    let all_ids: HashSet<&str> = photo_index.keys().map(|s: &String| s.as_str()).collect();

    let mut warnings = Vec::new();

    // Find orphaned placements (in layout but not in photos)
    let orphaned: Vec<&str> = placed_ids.difference(&all_ids).copied().collect();
    for id in &orphaned {
        for page in &state.layout {
            if page.photos.iter().any(|p| p == id) {
                warnings.push(format!(
                    "Orphaned placement: {} on page {} (not in photos)",
                    id, page.page
                ));
            }
        }
    }

    warnings
}

/// Check if two ratios are compatible (≤5% difference)
fn ratios_compatible(a: f64, b: f64) -> bool {
    let (min, max) = if a < b { (a, b) } else { (b, a) };
    if min > 0.0 {
        (max - min) / min <= 0.05
    } else {
        (max - min).abs() < 0.001
    }
}

/// Assign swap groups based on aspect ratios
/// Photos with compatible ratios (≤5% difference) get the same letter
fn assign_swap_groups(ratios: &[f64]) -> Vec<char> {
    if ratios.is_empty() {
        return vec![];
    }

    // Sort indices by ratio
    let mut indices: Vec<usize> = (0..ratios.len()).collect();
    indices.sort_by(|&a, &b| ratios[a].partial_cmp(&ratios[b]).unwrap_or(std::cmp::Ordering::Equal));

    let mut groups = vec![' '; ratios.len()];
    let mut current_group = b'A';
    groups[indices[0]] = current_group as char;

    for window in indices.windows(2) {
        let prev_ratio = ratios[window[0]];
        let curr_ratio = ratios[window[1]];
        if !ratios_compatible(prev_ratio, curr_ratio) {
            current_group += 1;
        }
        groups[window[1]] = current_group as char;
    }

    groups
}

/// Build detail information for a single page
fn build_page_detail(
    state: &ProjectState,
    page_num: usize,
    modified_pages: &[usize],
) -> Result<PageDetail> {
    if page_num == 0 || page_num > state.layout.len() {
        anyhow::bail!(
            "Invalid page {} (layout has {} pages)",
            page_num,
            state.layout.len()
        );
    }

    let page = &state.layout[page_num - 1];
    let photo_index = build_photo_index(&state.photos);

    // Collect ratios for all photos on this page
    let ratios: Vec<f64> = page
        .photos
        .iter()
        .map(|id| {
            photo_index
                .get(id.as_str())
                .map(|(pf, _): &(crate::dto_models::PhotoFile, String)| pf.aspect_ratio())
                .unwrap_or(1.0)
        })
        .collect();

    let swap_groups = assign_swap_groups(&ratios);

    // Check if this page was modified since last build
    let modified = modified_pages.contains(&page_num);

    let slots: Vec<SlotInfo> = page
        .photos
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let slot_mm = page
                .slots
                .get(i)
                .map(|s| (s.x_mm, s.y_mm, s.width_mm, s.height_mm))
                .unwrap_or((0.0, 0.0, 0.0, 0.0));
            SlotInfo {
                photo_id: id.clone(),
                ratio: ratios[i],
                swap_group: swap_groups[i],
                slot_mm,
            }
        })
        .collect();

    Ok(PageDetail {
        page: page_num,
        photo_count: page.photos.len(),
        modified,
        slots,
    })
}

/// Show project status (read-only)
pub fn status(project_root: &Path, config: &StatusConfig) -> Result<StatusReport> {
    let mgr = StateManager::open(project_root)?;
    let project_name = mgr.project_name().to_owned();

    // Basic numbers
    let total_photos: usize = mgr.state.photos.iter().map(|g| g.files.len()).sum();
    let group_count = mgr.state.photos.len();
    let unplaced = count_unplaced(&mgr.state);
    let page_count = mgr.state.layout.len();
    let avg = if page_count > 0 {
        total_photos as f64 / page_count as f64
    } else {
        0.0
    };

    // Determine project state and changed pages
    let (project_state, page_changes) = if mgr.state.layout.is_empty() {
        (ProjectState_::Empty, vec![])
    } else if !mgr.has_changes_since_last_build() {
        (ProjectState_::Clean, vec![])
    } else {
        let modified = mgr.modified_pages();
        if modified.is_empty() {
            (ProjectState_::Clean, vec![])
        } else {
            (ProjectState_::Modified, modified)
        }
    };

    // Consistency checks
    let warnings = check_consistency(&mgr.state);

    // Detail view (if page requested)
    let modified_pages = if mgr.state.layout.is_empty() {
        vec![]
    } else {
        mgr.modified_pages()
    };

    let detail = config
        .page
        .map(|p| build_page_detail(&mgr.state, p, &modified_pages))
        .transpose()?;

    Ok(StatusReport {
        project_name,
        state: project_state,
        total_photos,
        group_count,
        unplaced,
        page_count,
        avg_photos_per_page: avg,
        page_changes,
        detail,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_unplaced_all_placed() {
        let state = ProjectState {
            config: Default::default(),
            photos: vec![crate::dto_models::PhotoGroup {
                group: "Test".to_string(),
                sort_key: "2024-01-01".to_string(),
                files: vec![
                    crate::dto_models::PhotoFile {
                        id: "a.jpg".to_string(),
                        source: "/path/a.jpg".to_string(),
                        width_px: 1920,
                        height_px: 1080,
                        area_weight: 1.0,
                        timestamp: chrono::Utc::now(),
                        hash: "test".to_string(),
                    },
                ],
            }],
            layout: vec![crate::dto_models::LayoutPage {
                page: 1,
                photos: vec!["a.jpg".to_string()],
                slots: vec![],
            }],
        };

        assert_eq!(count_unplaced(&state), 0);
    }

    #[test]
    fn test_count_unplaced_some_unplaced() {
        let state = ProjectState {
            config: Default::default(),
            photos: vec![crate::dto_models::PhotoGroup {
                group: "Test".to_string(),
                sort_key: "2024-01-01".to_string(),
                files: vec![
                    crate::dto_models::PhotoFile {
                        id: "a.jpg".to_string(),
                        source: "/path/a.jpg".to_string(),
                        width_px: 1920,
                        height_px: 1080,
                        area_weight: 1.0,
                        timestamp: chrono::Utc::now(),
                        hash: "test".to_string(),
                    },
                    crate::dto_models::PhotoFile {
                        id: "b.jpg".to_string(),
                        source: "/path/b.jpg".to_string(),
                        width_px: 1920,
                        height_px: 1080,
                        area_weight: 1.0,
                        timestamp: chrono::Utc::now(),
                        hash: "test".to_string(),
                    },
                ],
            }],
            layout: vec![crate::dto_models::LayoutPage {
                page: 1,
                photos: vec!["a.jpg".to_string()],
                slots: vec![],
            }],
        };

        assert_eq!(count_unplaced(&state), 1);
    }

    #[test]
    fn test_ratios_compatible_within_tolerance() {
        assert!(ratios_compatible(1.0, 1.04));
        assert!(ratios_compatible(1.5, 1.575));
        assert!(!ratios_compatible(1.0, 1.06));
        assert!(!ratios_compatible(1.5, 1.6));
    }

    #[test]
    fn test_assign_swap_groups_single_group() {
        let ratios = vec![1.0, 1.02, 1.03];
        let groups = assign_swap_groups(&ratios);
        assert_eq!(groups, vec!['A', 'A', 'A']);
    }

    #[test]
    fn test_assign_swap_groups_multiple_groups() {
        let ratios = vec![0.67, 0.69, 1.5, 1.52];
        let groups = assign_swap_groups(&ratios);
        // Groups should have two distinct letters
        assert!(groups[0] == 'A' || groups[0] == 'B');
        assert!(groups[1] == groups[0]);
        assert!(groups[2] != groups[0]);
        assert!(groups[2] == groups[3]);
    }

    #[test]
    fn test_assign_swap_groups_empty() {
        let groups = assign_swap_groups(&[]);
        let expected: Vec<char> = vec![];
        assert_eq!(groups, expected);
    }

    #[test]
    fn test_check_consistency_no_orphans() {
        let state = ProjectState {
            config: Default::default(),
            photos: vec![crate::dto_models::PhotoGroup {
                group: "Test".to_string(),
                sort_key: "2024-01-01".to_string(),
                files: vec![crate::dto_models::PhotoFile {
                    id: "a.jpg".to_string(),
                    source: "/path/a.jpg".to_string(),
                    width_px: 1920,
                    height_px: 1080,
                    area_weight: 1.0,
                    timestamp: chrono::Utc::now(),
                    hash: "test".to_string(),
                }],
            }],
            layout: vec![crate::dto_models::LayoutPage {
                page: 1,
                photos: vec!["a.jpg".to_string()],
                slots: vec![],
            }],
        };

        let warnings = check_consistency(&state);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_check_consistency_orphaned() {
        let state = ProjectState {
            config: Default::default(),
            photos: vec![crate::dto_models::PhotoGroup {
                group: "Test".to_string(),
                sort_key: "2024-01-01".to_string(),
                files: vec![],
            }],
            layout: vec![crate::dto_models::LayoutPage {
                page: 1,
                photos: vec!["orphan.jpg".to_string()],
                slots: vec![],
            }],
        };

        let warnings = check_consistency(&state);
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("orphan.jpg"));
    }
}
