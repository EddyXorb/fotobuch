//! YAML file generation for project state

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::models::{BookConfig, CoverConfig, ProjectConfig, ProjectState, SpineConfig};
use tracing::warn;

use super::NewConfig;

/// Generate default project state from a `NewConfig`.
///
/// If `config.base_config` is set it is used as the starting point; the
/// dimension, cover and title fields from `NewConfig` are then overwritten on
/// top of it. Otherwise defaults are used.
pub fn generate_default_state(config: &NewConfig) -> ProjectState {
    let NewConfig {
        name,
        width_mm,
        height_mm,
        bleed_mm,
        with_cover,
        cover_width_mm,
        cover_height_mm,
        spine_grow_per_10_pages_mm,
        spine_mm,
        margin_mm,
        base_config,
        ..
    } = config;

    let base = base_config.clone().unwrap_or_default();

    let cover = if *with_cover {
        let cw = cover_width_mm.unwrap_or_else(|| {
            warn!("cover active but no cover width given, using page width * 2");
            width_mm * 2.0
        });
        let ch = cover_height_mm.unwrap_or_else(|| {
            warn!("cover active but no cover height given, using page height");
            *height_mm
        });
        let spine_config = if let Some(rate) = spine_grow_per_10_pages_mm {
            SpineConfig::Auto {
                spine_mm_per_10_pages: *rate,
            }
        } else {
            SpineConfig::Fixed {
                spine_width_mm: spine_mm.expect("validated by ProjectError::CoverWithoutSpine"),
            }
        };
        CoverConfig {
            active: true,
            mode: Default::default(),
            spine_clearance_mm: 5.0,
            spine: spine_config,
            front_back_width_mm: cw,
            height_mm: ch,
            spine_text: None,
            bleed_mm: *bleed_mm,
            margin_mm: 0.0,
            gap_mm: 5.0,
            bleed_threshold_mm: 3.0,
        }
    } else {
        CoverConfig::default()
    };

    ProjectState {
        config: ProjectConfig {
            book: BookConfig {
                title: name.clone(),
                page_width_mm: *width_mm,
                page_height_mm: *height_mm,
                bleed_mm: *bleed_mm,
                margin_mm: *margin_mm,
                cover,
                ..base.book
            },
            ..base
        },
        photos: Vec::new(),
        layout: Vec::new(),
    }
}

/// Write project state to YAML file
pub fn write_yaml(path: &Path, state: &ProjectState) -> Result<()> {
    let yaml_string =
        serde_yaml::to_string(state).context("Failed to serialize project state to YAML")?;

    fs::write(path, yaml_string).with_context(|| format!("Failed to write YAML to {:?}", path))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::NewConfig;
    use super::*;
    use tempfile::TempDir;

    fn test_config() -> NewConfig {
        NewConfig {
            name: "test".to_string(),
            width_mm: 210.0,
            height_mm: 297.0,
            bleed_mm: 3.0,
            with_cover: false,
            cover_width_mm: None,
            cover_height_mm: None,
            spine_grow_per_10_pages_mm: None,
            spine_mm: None,
            margin_mm: 0.0,
            base_config: None,
        }
    }

    #[test]
    fn test_generate_default_state() {
        let state = generate_default_state(&test_config());

        assert_eq!(state.config.book.page_width_mm, 210.0);
        assert_eq!(state.config.book.page_height_mm, 297.0);
        assert_eq!(state.config.book.bleed_mm, 3.0);
        assert!(!state.config.book.cover.active);
        assert!(state.photos.is_empty());
        assert!(state.layout.is_empty());
    }

    #[test]
    fn test_base_config_preserves_non_dimension_fields() {
        use crate::models::{BookConfig, PreviewConfig, ProjectConfig};

        let base = ProjectConfig {
            book: BookConfig {
                dpi: 150.0, // a field that should survive untouched
                ..Default::default()
            },
            preview: PreviewConfig {
                show_slot_info: false,
                show_preview_watermark: false,
                show_borders: false,
                show_filenames: false,
                ..Default::default()
            },
            ..Default::default()
        };

        let config = NewConfig {
            width_mm: 100.0,
            base_config: Some(base),
            ..test_config()
        };

        let state = generate_default_state(&config);

        // Dimensions from NewConfig overwrite the base.
        assert_eq!(state.config.book.page_width_mm, 100.0);
        // Non-dimension book and preview fields come from the base.
        assert_eq!(state.config.book.dpi, 150.0);
        assert!(!state.config.preview.show_slot_info);
        assert!(!state.config.preview.show_preview_watermark);
        assert!(!state.config.preview.show_borders);
        assert!(!state.config.preview.show_filenames);
    }

    #[test]
    fn test_default_base_config_uses_defaults() {
        let state = generate_default_state(&test_config());

        assert_eq!(state.config.book.gap_mm, 5.0);
        assert_eq!(state.config.book.dpi, 300.0);
        assert!(state.config.preview.show_slot_info);
    }

    #[test]
    fn test_write_yaml() {
        let temp_dir = TempDir::new();
        let temp_dir = temp_dir.unwrap();
        let yaml_path = temp_dir.path().join("test.yaml");

        let state = generate_default_state(&test_config());
        write_yaml(&yaml_path, &state).unwrap();

        assert!(yaml_path.exists());

        let content = fs::read_to_string(&yaml_path).unwrap();
        assert!(content.contains("page_width_mm: 210"));
        assert!(content.contains("page_height_mm: 297"));
        assert!(content.contains("bleed_mm: 3"));
    }
}
