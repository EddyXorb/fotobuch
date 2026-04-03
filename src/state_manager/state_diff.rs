use std::collections::HashSet;

use serde_yaml::Value;

use crate::dto_models::{LayoutPage, PhotoGroup, ProjectState};

/// Summary of differences between two [`ProjectState`] snapshots.
#[derive(Debug, Default, PartialEq)]
pub(super) struct StateDiff {
    pub config_changes: usize,
    pub photos_added: usize,
    pub photos_removed: usize,
    pub photos_modified: usize,
    pub pages_added: usize,
    pub pages_removed: usize,
    pub pages_modified: usize,
}

impl StateDiff {
    /// Compute the diff between `old` and `new`.
    pub fn compute(old: &ProjectState, new: &ProjectState) -> Self {
        let config_changes = count_config_changes(old, new);

        let (photos_added, photos_removed, photos_modified) = diff_photos(&old.photos, &new.photos);
        let (pages_added, pages_removed, pages_modified) = diff_pages(&old.layout, &new.layout);

        Self {
            config_changes,
            photos_added,
            photos_removed,
            photos_modified,
            pages_added,
            pages_removed,
            pages_modified,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.config_changes == 0
            && self.photos_added == 0
            && self.photos_removed == 0
            && self.photos_modified == 0
            && self.pages_added == 0
            && self.pages_removed == 0
            && self.pages_modified == 0
    }

    /// Human-readable one-line summary, e.g. `"changed 2 configs, added 15 photos"`.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.config_changes > 0 {
            parts.push(format!("changed {} config(s)", self.config_changes));
        }
        if self.photos_added > 0 {
            parts.push(format!("added {} photo(s)", self.photos_added));
        }
        if self.photos_removed > 0 {
            parts.push(format!("removed {} photo(s)", self.photos_removed));
        }
        if self.photos_modified > 0 {
            parts.push(format!("modified {} photo(s)", self.photos_modified));
        }
        if self.pages_added > 0 {
            parts.push(format!("added {} page(s)", self.pages_added));
        }
        if self.pages_removed > 0 {
            parts.push(format!("removed {} page(s)", self.pages_removed));
        }
        if self.pages_modified > 0 {
            parts.push(format!("modified {} page(s)", self.pages_modified));
        }
        if parts.is_empty() {
            "no changes".to_owned()
        } else {
            parts.join(", ")
        }
    }
}

/// Count differing leaf values in the config section by serialising both states
/// to `serde_yaml::Value` and recursively comparing leaves.
pub fn count_config_changes(old: &ProjectState, new: &ProjectState) -> usize {
    let old_val = serde_yaml::to_value(&old.config).unwrap_or(Value::Null);
    let new_val = serde_yaml::to_value(&new.config).unwrap_or(Value::Null);
    count_value_diffs(&old_val, &new_val)
}

pub fn count_value_diffs(a: &Value, b: &Value) -> usize {
    match (a, b) {
        (Value::Mapping(ma), Value::Mapping(mb)) => {
            let keys: HashSet<_> = ma.keys().chain(mb.keys()).collect();
            keys.into_iter()
                .map(|k| {
                    count_value_diffs(
                        ma.get(k).unwrap_or(&Value::Null),
                        mb.get(k).unwrap_or(&Value::Null),
                    )
                })
                .sum()
        }
        _ => usize::from(a != b),
    }
}

/// Returns (added, removed, modified) photo counts.
///
/// Modified = same photo ID but different `area_weight` or pixel dimensions.
pub fn diff_photos(old: &[PhotoGroup], new: &[PhotoGroup]) -> (usize, usize, usize) {
    let old_map: std::collections::HashMap<&str, &crate::dto_models::PhotoFile> = old
        .iter()
        .flat_map(|g| g.files.iter().map(|f| (f.id.as_str(), f)))
        .collect();
    let new_map: std::collections::HashMap<&str, &crate::dto_models::PhotoFile> = new
        .iter()
        .flat_map(|g| g.files.iter().map(|f| (f.id.as_str(), f)))
        .collect();

    let old_ids: HashSet<&str> = old_map.keys().copied().collect();
    let new_ids: HashSet<&str> = new_map.keys().copied().collect();

    let added = new_ids.difference(&old_ids).count();
    let removed = old_ids.difference(&new_ids).count();
    let modified = old_ids
        .intersection(&new_ids)
        .filter(|&&id| {
            let o = old_map[id];
            let n = new_map[id];
            o.area_weight != n.area_weight || o.width_px != n.width_px || o.height_px != n.height_px
        })
        .count();

    (added, removed, modified)
}

/// Returns (pages_added, pages_removed, pages_modified).
///
/// Modified = a page that exists in both old and new but has different slots.
pub fn diff_pages(old: &[LayoutPage], new: &[LayoutPage]) -> (usize, usize, usize) {
    let old_map: std::collections::HashMap<usize, &LayoutPage> =
        old.iter().map(|p| (p.page, p)).collect();
    let new_map: std::collections::HashMap<usize, &LayoutPage> =
        new.iter().map(|p| (p.page, p)).collect();

    let added = new_map.keys().filter(|k| !old_map.contains_key(k)).count();
    let removed = old_map.keys().filter(|k| !new_map.contains_key(k)).count();
    let modified = old_map
        .iter()
        .filter(|(k, old_page)| {
            new_map.get(k).is_some_and(|new_page| {
                old_page.slots != new_page.slots
                    || old_page.photos != new_page.photos
                    || old_page.mode != new_page.mode
            })
        })
        .count();

    (added, removed, modified)
}
