use std::collections::HashMap;

use crate::models::{PhotoFile, PhotoGroup, ProjectState, build_photo_index};

/// Collects all photos from the page range and reconstructs PhotoGroups.
///
/// # Arguments
/// * `start` - **0-based** index (inclusive), e.g., 0 for the first page
/// * `end` - Slice end (exclusive), e.g., 2 to include pages at indices 0 and 1
pub fn collect_photos_as_groups(state: &ProjectState, start: usize, end: usize) -> Vec<PhotoGroup> {
    let photo_index = build_photo_index(&state.photos);

    // Collect photo IDs from the page range
    let page_photo_ids: Vec<&str> = state.layout[start..end]
        .iter()
        .flat_map(|p| p.photos.iter().map(String::as_str))
        .collect();

    // Split by original group
    let mut groups_map: HashMap<&str, Vec<PhotoFile>> = HashMap::new();
    for id in &page_photo_ids {
        if let Some((pf, group_name)) = photo_index.get(*id) {
            groups_map
                .entry(group_name)
                .or_default()
                .push((*pf).clone());
        }
    }

    // sort_key aus photos übernehmen
    let group_sort_keys: HashMap<&str, &str> = state
        .photos
        .iter()
        .map(|g| (g.group.as_str(), g.sort_key.as_str()))
        .collect();

    let mut groups: Vec<PhotoGroup> = groups_map
        .into_iter()
        .map(|(name, files)| PhotoGroup {
            group: name.to_string(),
            sort_key: group_sort_keys.get(name).unwrap_or(&"").to_string(),
            files,
        })
        .collect();

    groups.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));
    groups
}
