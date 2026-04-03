//! Project state structures for fotobuch.yaml

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::dto_models::*;

/// Complete project state as persisted in fotobuch.yaml
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectState {
    /// Configuration (page dimensions, GA settings, etc.)
    pub config: ProjectConfig,
    /// Imported photos grouped by directory
    pub photos: Vec<PhotoGroup>,
    /// Calculated layout (pages with photos and slots)
    #[serde(default)]
    pub layout: Vec<LayoutPage>,
}

impl ProjectState {
    /// Load project state from fotobuch.yaml
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let state: ProjectState = serde_yaml::from_str(&contents)
            .with_context(|| format!("Failed to parse YAML from {}", path.display()))?;

        Ok(state)
    }

    /// Save project state to fotobuch.yaml
    pub fn save(&self, path: &Path) -> Result<()> {
        let yaml =
            serde_yaml::to_string(self).context("Failed to serialize project state to YAML")?;

        std::fs::write(path, yaml)
            .with_context(|| format!("Failed to write {}", path.display()))?;

        Ok(())
    }

    /// Returns true if the cover page is configured and active.
    pub fn has_cover(&self) -> bool {
        self.config.book.cover.active
    }

    pub fn check_validity(&self) -> Result<()> {
        // Build known photo IDs, checking for duplicates within the photos section
        let mut known_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for group in &self.photos {
            for file in &group.files {
                if !known_ids.insert(file.id.as_str()) {
                    return Err(anyhow::anyhow!(
                        "Duplicate photo ID in photos section: {}",
                        file.id
                    ));
                }
            }
        }

        let mut layout_photo_ids: std::collections::HashSet<&str> =
            std::collections::HashSet::new();

        for (i, page) in self.layout.iter().enumerate() {
            if page.page != i {
                return Err(anyhow::anyhow!(
                    "Page at position {} has index {}, expected {}",
                    i,
                    page.page,
                    i
                ));
            }

            if page.photos.len() != page.slots.len() {
                return Err(anyhow::anyhow!(
                    "Page {}: {} photo(s) but {} slot(s)",
                    i,
                    page.photos.len(),
                    page.slots.len()
                ));
            }

            for photo_id in &page.photos {
                if !known_ids.contains(photo_id.as_str()) {
                    return Err(anyhow::anyhow!(
                        "Page {}: photo '{}' not found in photos section",
                        i,
                        photo_id
                    ));
                }
                if (!self.has_cover() || i > 0) && !layout_photo_ids.insert(photo_id.as_str()) {
                    return Err(anyhow::anyhow!(
                        "Photo '{}' appears on more than one page",
                        photo_id
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_photo(id: &str) -> PhotoFile {
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

    fn make_slot() -> Slot {
        Slot {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: 100.0,
            height_mm: 66.67,
        }
    }

    fn make_page(page: usize, photo_ids: &[&str]) -> LayoutPage {
        LayoutPage {
            page,
            photos: photo_ids.iter().map(|s| s.to_string()).collect(),
            slots: photo_ids.iter().map(|_| make_slot()).collect(),
            mode: None,
        }
    }

    fn minimal_state() -> ProjectState {
        ProjectState {
            config: Default::default(),
            photos: vec![PhotoGroup {
                group: "G".into(),
                sort_key: "2024-01-15T00:00:00Z".into(),
                files: vec![make_photo("G/a.jpg"), make_photo("G/b.jpg")],
            }],
            layout: vec![make_page(0, &["G/a.jpg", "G/b.jpg"])],
        }
    }

    #[test]
    fn test_validity_ok() {
        assert!(minimal_state().check_validity().is_ok());
    }

    #[test]
    fn test_validity_photo_not_in_photos_section() {
        let mut state = minimal_state();
        state.layout[0].photos[0] = "G/unknown.jpg".into();
        let err = state.check_validity().unwrap_err();
        assert!(err.to_string().contains("not found in photos section"));
    }

    #[test]
    fn test_validity_photos_slots_count_mismatch() {
        let mut state = minimal_state();
        state.layout[0].slots.pop();
        let err = state.check_validity().unwrap_err();
        assert!(err.to_string().contains("slot(s)"));
    }

    #[test]
    fn test_validity_page_index_mismatch() {
        let mut state = minimal_state();
        state.layout[0].page = 1;
        let err = state.check_validity().unwrap_err();
        assert!(err.to_string().contains("expected 0"));
    }

    #[test]
    fn test_validity_duplicate_photo_in_photos_section() {
        let mut state = minimal_state();
        state.photos[0].files.push(make_photo("G/a.jpg"));
        let err = state.check_validity().unwrap_err();
        assert!(err.to_string().contains("Duplicate photo ID"));
    }

    #[test]
    fn test_validity_photo_on_multiple_pages() {
        let mut state = minimal_state();
        state.layout.push(make_page(1, &["G/a.jpg"]));
        let err = state.check_validity().unwrap_err();
        assert!(err.to_string().contains("appears on more than one page"));
    }

    #[test]
    fn test_serialize_deserialize() {
        let state = ProjectState {
            config: ProjectConfig {
                book: crate::dto_models::BookConfig {
                    title: "Test".into(),
                    page_width_mm: 420.0,
                    page_height_mm: 297.0,
                    bleed_mm: 3.0,
                    margin_mm: 10.0,
                    gap_mm: 5.0,
                    bleed_threshold_mm: 3.0,
                    dpi: 300.0,
                    cover: Default::default(),
                    appendix: Default::default(),
                },
                page_layout_solver: Default::default(),
                preview: Default::default(),
                book_layout_solver: BookLayoutSolverConfig::default(),
            },
            photos: vec![PhotoGroup {
                group: "TestGroup".into(),
                sort_key: "2024-01-15T00:00:00Z".into(),
                files: vec![PhotoFile {
                    id: "TestGroup/photo1.jpg".into(),
                    source: "/path/to/photo1.jpg".into(),
                    timestamp: "2024-01-15T00:00:00Z".parse().unwrap(),
                    width_px: 6000,
                    height_px: 4000,
                    area_weight: 1.0,
                    hash: String::new(),
                }],
            }],
            layout: vec![],
        };

        // Serialize
        let yaml = serde_yaml::to_string(&state).unwrap();
        assert!(yaml.contains("TestGroup"));
        assert!(yaml.contains("photo1.jpg"));

        // Deserialize
        let deserialized: ProjectState = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(deserialized.photos.len(), 1);
        assert_eq!(deserialized.photos[0].files.len(), 1);
        assert_eq!(deserialized.photos[0].files[0].id, "TestGroup/photo1.jpg");
    }

    #[test]
    fn test_load_save_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_path = temp_dir.path().join("fotobuch.yaml");

        let state = ProjectState {
            config: ProjectConfig {
                book: crate::dto_models::BookConfig {
                    title: "Test".into(),
                    page_width_mm: 420.0,
                    page_height_mm: 297.0,
                    bleed_mm: 3.0,
                    margin_mm: 10.0,
                    gap_mm: 5.0,
                    bleed_threshold_mm: 3.0,
                    dpi: 300.0,
                    cover: Default::default(),
                    appendix: Default::default(),
                },
                page_layout_solver: Default::default(),
                preview: Default::default(),
                book_layout_solver: BookLayoutSolverConfig::default(),
            },
            photos: vec![],
            layout: vec![],
        };

        // Save
        state.save(&yaml_path).unwrap();
        assert!(yaml_path.exists());

        // Load
        let loaded = ProjectState::load(&yaml_path).unwrap();
        assert_eq!(loaded.config.book.page_width_mm, 420.0);
    }
}
