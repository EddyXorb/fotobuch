//! Project state structures for fotobuch.yaml

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::models::ProjectConfig;

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

/// Group of photos from a single directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoGroup {
    /// Group name (relative path from add argument)
    pub group: String,
    /// Timestamp for chronological ordering (ISO 8601)
    pub sort_key: String,
    /// Photos in this group
    pub files: Vec<PhotoFile>,
}

/// Individual photo with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoFile {
    /// Unique photo ID (used in layout)
    pub id: String,
    /// Absolute path to original file
    pub source: String,
    /// Width in pixels
    pub width_px: u32,
    /// Height in pixels
    pub height_px: u32,
    /// Area weight for solver (default: 1.0)
    #[serde(default = "default_area_weight")]
    pub area_weight: f64,
    /// Hash for duplicate detection (not serialized to YAML)
    #[serde(skip)]
    pub hash: Option<String>,
}

fn default_area_weight() -> f64 {
    1.0
}

/// Single page in the layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPage {
    /// Page number (1-based, for user reference only)
    pub page: usize,
    /// Photo IDs on this page (sorted by ratio)
    pub photos: Vec<String>,
    /// Calculated slot positions (index-coupled to photos)
    pub slots: Vec<Slot>,
}

/// Placement slot for a photo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    /// X position in mm
    pub x_mm: f64,
    /// Y position in mm
    pub y_mm: f64,
    /// Width in mm
    pub width_mm: f64,
    /// Height in mm
    pub height_mm: f64,
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
        let yaml = serde_yaml::to_string(self)
            .context("Failed to serialize project state to YAML")?;

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
                book: crate::models::BookConfig {
                    title: "Test".into(),
                    page_width_mm: 420.0,
                    page_height_mm: 297.0,
                    bleed_mm: 3.0,
                    margin_mm: 10.0,
                    gap_mm: 5.0,
                    bleed_threshold_mm: 3.0,
                },
                ga: Default::default(),
                preview: Default::default(),
            },
            photos: vec![PhotoGroup {
                group: "TestGroup".into(),
                sort_key: "2024-01-15T00:00:00Z".into(),
                files: vec![PhotoFile {
                    id: "TestGroup/photo1.jpg".into(),
                    source: "/path/to/photo1.jpg".into(),
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
                book: crate::models::BookConfig {
                    title: "Test".into(),
                    page_width_mm: 420.0,
                    page_height_mm: 297.0,
                    bleed_mm: 3.0,
                    margin_mm: 10.0,
                    gap_mm: 5.0,
                    bleed_threshold_mm: 3.0,
                },
                ga: Default::default(),
                preview: Default::default(),
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
