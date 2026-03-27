//! YAML file generation for project state

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::dto_models::{
    BookConfig, BookLayoutSolverConfig, CoverConfig, GaConfig, PreviewConfig, ProjectConfig,
    ProjectState, SpineConfig,
};
use tracing::warn;

use super::NewConfig;

/// Generate default project state from a `NewConfig`.
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
        ..
    } = config;

    let cover = if *with_cover {
        let cw = cover_width_mm.unwrap_or_else(|| {
            warn!("--with-cover set but --cover-width not provided, using page_width * 2");
            width_mm * 2.0
        });
        let ch = cover_height_mm.unwrap_or_else(|| {
            warn!("--with-cover set but --cover-height not provided, using page_height");
            *height_mm
        });
        let spine_config = if let Some(rate) = spine_grow_per_10_pages_mm {
            SpineConfig::Auto {
                spine_mm_per_10_pages: *rate,
            }
        } else {
            SpineConfig::Fixed {
                spine_width_mm: spine_mm.expect("validated in CLI handler"),
            }
        };
        CoverConfig {
            active: true,
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
                margin_mm: 10.0,
                gap_mm: 5.0,
                bleed_threshold_mm: 3.0,
                dpi: 300.0,
                cover,
            },
            page_layout_solver: GaConfig::default(),
            preview: PreviewConfig::default(),
            book_layout_solver: BookLayoutSolverConfig::default(),
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
            quiet: false,
            with_cover: false,
            cover_width_mm: None,
            cover_height_mm: None,
            spine_grow_per_10_pages_mm: None,
            spine_mm: None,
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
    fn test_write_yaml() {
        let temp_dir = TempDir::new().unwrap();
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
