use std::collections::HashMap;

use fotobuch::dto_models::{PhotoFile, ProjectState};

/// Derived lookup tables computed from [`ProjectState`] on startup and after each command.
///
/// `rebuild` does a single pass over `photos` and a single pass over `layout` — no incremental
/// patching, no background thread in Phase 2.
// Fields used in Phase 3+ (commands, photo pool). Phase 2 only reads `unplaced_photos`.
#[allow(dead_code)]
pub struct DerivedState {
    /// All photos indexed by ID for O(1) lookup.
    pub photo_by_id: HashMap<String, PhotoFile>,
    /// For each placed photo: `(page_idx, slot_idx)`.
    pub placement_of_photo: HashMap<String, (usize, usize)>,
    /// Photo IDs that appear in `photos` but not in any layout page.
    pub unplaced_photos: Vec<String>,
    /// Maps photo ID → group name.
    pub group_of_photo: HashMap<String, String>,
    /// How many photos from each group are currently placed.
    pub placed_per_group: HashMap<String, usize>,
}

impl DerivedState {
    pub fn rebuild(project: &ProjectState) -> Self {
        let mut photo_by_id: HashMap<String, PhotoFile> = HashMap::new();
        let mut group_of_photo: HashMap<String, String> = HashMap::new();

        for group in &project.photos {
            for file in &group.files {
                photo_by_id.insert(file.id.clone(), file.clone());
                group_of_photo.insert(file.id.clone(), group.group.clone());
            }
        }

        let mut placement_of_photo: HashMap<String, (usize, usize)> = HashMap::new();
        let mut placed_per_group: HashMap<String, usize> = HashMap::new();

        for layout_page in &project.layout {
            for (slot_idx, photo_id) in layout_page.photos.iter().enumerate() {
                placement_of_photo.insert(photo_id.clone(), (layout_page.page, slot_idx));
                if let Some(group) = group_of_photo.get(photo_id) {
                    *placed_per_group.entry(group.clone()).or_insert(0) += 1;
                }
            }
        }

        let mut unplaced_photos: Vec<String> = photo_by_id
            .keys()
            .filter(|id| !placement_of_photo.contains_key(*id))
            .cloned()
            .collect();
        unplaced_photos.sort();

        Self {
            photo_by_id,
            placement_of_photo,
            unplaced_photos,
            group_of_photo,
            placed_per_group,
        }
    }
}

#[cfg(test)]
mod tests {
    use fotobuch::dto_models::{
        LayoutPage, PageMode, PhotoFile, PhotoGroup, ProjectConfig, ProjectState, Slot,
    };

    use super::*;

    fn photo(id: &str) -> PhotoFile {
        PhotoFile {
            id: id.into(),
            source: format!("/path/{id}"),
            timestamp: "2024-01-15T00:00:00Z".parse().unwrap(),
            width_px: 6000,
            height_px: 4000,
            area_weight: 1.0,
            hash: String::new(),
        }
    }

    fn slot() -> Slot {
        Slot {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: 100.0,
            height_mm: 66.0,
        }
    }

    fn empty_project() -> ProjectState {
        ProjectState {
            config: ProjectConfig::default(),
            photos: vec![],
            layout: vec![],
        }
    }

    fn project_with_photos() -> ProjectState {
        ProjectState {
            config: ProjectConfig::default(),
            photos: vec![
                PhotoGroup {
                    group: "A".into(),
                    sort_key: "2024-01-15T00:00:00Z".into(),
                    files: vec![photo("A/1.jpg"), photo("A/2.jpg")],
                },
                PhotoGroup {
                    group: "B".into(),
                    sort_key: "2024-01-16T00:00:00Z".into(),
                    files: vec![photo("B/3.jpg")],
                },
            ],
            layout: vec![LayoutPage {
                page: 0,
                photos: vec!["A/1.jpg".into(), "B/3.jpg".into()],
                slots: vec![slot(), slot()],
                mode: PageMode::Auto,
            }],
        }
    }

    #[test]
    fn rebuild_empty_state() {
        let derived = DerivedState::rebuild(&empty_project());
        assert!(derived.photo_by_id.is_empty());
        assert!(derived.placement_of_photo.is_empty());
        assert!(derived.unplaced_photos.is_empty());
        assert!(derived.group_of_photo.is_empty());
        assert!(derived.placed_per_group.is_empty());
    }

    #[test]
    fn rebuild_places_photos_into_placement_map() {
        let derived = DerivedState::rebuild(&project_with_photos());
        assert_eq!(derived.photo_by_id.len(), 3);
        assert_eq!(derived.placement_of_photo.get("A/1.jpg"), Some(&(0, 0)));
        assert_eq!(derived.placement_of_photo.get("B/3.jpg"), Some(&(0, 1)));
        assert_eq!(derived.unplaced_photos, vec!["A/2.jpg"]);
    }

    #[test]
    fn rebuild_group_lookup_resolves_each_photo_to_its_group() {
        let derived = DerivedState::rebuild(&project_with_photos());
        assert_eq!(
            derived.group_of_photo.get("A/1.jpg").map(String::as_str),
            Some("A")
        );
        assert_eq!(
            derived.group_of_photo.get("A/2.jpg").map(String::as_str),
            Some("A")
        );
        assert_eq!(
            derived.group_of_photo.get("B/3.jpg").map(String::as_str),
            Some("B")
        );
        assert_eq!(derived.placed_per_group.get("A"), Some(&1));
        assert_eq!(derived.placed_per_group.get("B"), Some(&1));
    }
}
