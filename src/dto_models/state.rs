//! Project state structures for fotobuch.yaml

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::dto_models::*;

/// Complete project state as persisted in fotobuch.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
                    hash: None,
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
