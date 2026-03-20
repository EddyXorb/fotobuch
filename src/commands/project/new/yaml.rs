//! YAML file generation for project state

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::dto_models::{
    BookConfig, BookLayoutSolverConfig, GaConfig, PreviewConfig, ProjectConfig, ProjectState,
};

/// Generate default project state with given dimensions
pub fn generate_default_state(
    name: &str,
    width_mm: f64,
    height_mm: f64,
    bleed_mm: f64,
) -> ProjectState {
    ProjectState {
        config: ProjectConfig {
            book: BookConfig {
                title: name.to_string(),
                page_width_mm: width_mm,
                page_height_mm: height_mm,
                bleed_mm,
                margin_mm: 10.0,
                gap_mm: 5.0,
                bleed_threshold_mm: 3.0,
                dpi: 300.0,
                cover: Default::default(),
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
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_default_state() {
        let state = generate_default_state("test", 210.0, 297.0, 3.0);

        assert_eq!(state.config.book.page_width_mm, 210.0);
        assert_eq!(state.config.book.page_height_mm, 297.0);
        assert_eq!(state.config.book.bleed_mm, 3.0);
        assert!(state.photos.is_empty());
        assert!(state.layout.is_empty());
    }

    #[test]
    fn test_write_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_path = temp_dir.path().join("test.yaml");

        let state = generate_default_state("test", 210.0, 297.0, 3.0);
        write_yaml(&yaml_path, &state).unwrap();

        assert!(yaml_path.exists());

        let content = fs::read_to_string(&yaml_path).unwrap();
        assert!(content.contains("page_width_mm: 210"));
        assert!(content.contains("page_height_mm: 297"));
        assert!(content.contains("bleed_mm: 3"));
    }
}
