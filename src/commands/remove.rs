//! `fotobuch remove` command - Remove photos or groups from the project

mod matchers;
use matchers::{collect_unplaced_ids, match_photos, remove_from_layout, remove_from_photos};

use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

use crate::commands::page::delete_empty_pages;
use crate::commands::{CommandOutput, run_write_command};

/// Determines which photos to remove.
#[derive(Debug, Clone)]
pub enum RemoveTarget {
    /// Match by group name (exact) or regex on photo.source.
    Patterns(Vec<String>),
    /// Match by exact photo IDs (GUI Pool-Delete).
    Ids(Vec<String>),
    /// All photos not placed in any layout page.
    Unplaced,
}

/// Configuration for removing photos
#[derive(Debug, Clone)]
pub struct RemoveConfig {
    pub target: RemoveTarget,
    /// Only remove from layout, keep in photos (makes them unplaced)
    pub keep_files: bool,
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
pub fn remove(project_root: &Path, config: &RemoveConfig) -> Result<CommandOutput<RemoveResult>> {
    run_write_command(project_root, |mgr| {
        // 1. Determine which IDs to act on
        let (matched_ids, matched_groups) = match &config.target {
            RemoveTarget::Patterns(patterns) => {
                let matches = match_photos(mgr.state(), patterns)?;
                (matches.matched_ids, matches.matched_groups)
            }
            RemoveTarget::Ids(ids) => {
                let matched_ids: HashSet<String> = ids.iter().cloned().collect();
                (matched_ids, vec![])
            }
            RemoveTarget::Unplaced => (collect_unplaced_ids(mgr.state()), vec![]),
        };

        if matched_ids.is_empty() {
            return Ok((
                String::new(),
                RemoveResult {
                    photos_removed: 0,
                    placements_removed: 0,
                    groups_removed: vec![],
                    pages_affected: vec![],
                },
            ));
        }

        // 2. Aus Layout entfernen (immer, auch bei --keep-files)
        let layout_result = {
            let mut view = mgr.get_write_layout_state();
            let result = remove_from_layout(view.layout_mut(), &matched_ids);
            delete_empty_pages(view.layout_mut());
            result
        };

        // 4. Aus Photos entfernen (nur ohne --keep-files)
        let mut groups_removed = matched_groups;
        let photos_removed = if config.keep_files {
            0
        } else {
            let mut view = mgr.get_write_photos_state();
            remove_from_photos(view.photos_mut(), &matched_ids, &mut groups_removed)
        };

        let commit_msg = if matches!(config.target, RemoveTarget::Unplaced) {
            format!("remove: {} unplaced photos", photos_removed)
        } else if config.keep_files {
            format!(
                "remove: {} placements from layout (photos kept)",
                layout_result.placements_removed
            )
        } else {
            format!("remove: {} photos", photos_removed)
        };

        Ok((
            commit_msg,
            RemoveResult {
                photos_removed,
                placements_removed: layout_result.placements_removed,
                groups_removed,
                pages_affected: layout_result.pages_affected,
            },
        ))
    })
}
