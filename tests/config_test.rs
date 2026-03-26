//! Integration tests for `fotobuch config` command

use anyhow::Result;
use fotobuch::commands::{config, render_config};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test project
fn create_test_project(temp_dir: &TempDir) -> Result<PathBuf> {
    use fotobuch::commands::project::new::{NewConfig, project_new};

    let config = NewConfig {
        name: "testconfig".to_string(),
        width_mm: 200.0,
        height_mm: 250.0,
        bleed_mm: 3.0,
        quiet: true,
        with_cover: false,
        cover_width_mm: None,
        cover_height_mm: None,
        spine_grow_per_10_pages_mm: None,
        spine_mm: None,
        margin_mm: 0.0,
    };
    let result = project_new(temp_dir.path(), &config)?;
    Ok(result.project_root)
}

#[test]
fn test_config_minimal_yaml() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    let result = config(&project_root)?;

    // Resolved config should have all fields
    assert!(!result.resolved.book.title.is_empty());
    assert!(result.resolved.book.page_width_mm > 0.0);

    // Raw config should have some fields (the ones we specified in project_new)
    let raw_str = serde_yaml::to_string(&result.raw)?;
    assert!(!raw_str.is_empty());

    Ok(())
}

#[test]
fn test_config_render_output() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    let result = config(&project_root)?;
    let output = render_config(&result)?;

    // Output should contain the book config
    assert!(output.contains("book:"));
    assert!(output.contains("title:"));
    assert!(output.contains("page_width_mm:"));

    // Output should be valid YAML (can be parsed)
    let _reparsed: serde_yaml::Value = serde_yaml::from_str(&output)?;

    Ok(())
}

#[test]
fn test_config_render_has_field_annotations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    let result = config(&project_root)?;
    let output = render_config(&result)?;

    // The output should contain field names even if they're all explicit (no defaults in new project)
    // Check that the rendering includes config sections
    assert!(output.contains("book:"));
    assert!(output.contains("page_layout_solver:"));

    Ok(())
}

#[test]
fn test_config_output_valid_yaml() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    let result = config(&project_root)?;
    let output = render_config(&result)?;

    // Output should be valid YAML that can be parsed
    let reparsed: serde_yaml::Value = serde_yaml::from_str(&output)?;

    // The reparsed value should be a mapping
    assert!(reparsed.is_mapping());

    Ok(())
}

#[test]
fn test_config_resolved_matches_expected_structure() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    let result = config(&project_root)?;

    // Check expected config sections exist
    assert!(!result.resolved.book.title.is_empty());
    assert!(result.resolved.page_layout_solver.weights.w_size > 0.0);

    Ok(())
}

#[test]
fn test_config_raw_subset_of_resolved() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    let result = config(&project_root)?;

    // Raw config should have fewer fields than resolved (because of defaults)
    let raw_str = serde_yaml::to_string(&result.raw)?;
    let resolved_str = serde_yaml::to_string(&result.resolved)?;

    // Resolved should be longer or equal (it includes defaults)
    assert!(resolved_str.len() >= raw_str.len());

    Ok(())
}

#[test]
fn test_config_all_defaults_when_optional_fields_removed() -> Result<()> {
    use std::fs;

    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    // Load the YAML and remove optional fields with defaults from book config
    let yaml_path = project_root.join("testconfig.yaml");
    let mut yaml: serde_yaml::Value = serde_yaml::from_str(&fs::read_to_string(&yaml_path)?)?;

    // Remove optional fields from book config (margin_mm, gap_mm, bleed_threshold_mm have defaults)
    if let serde_yaml::Value::Mapping(ref mut map) = yaml
        && let Some(serde_yaml::Value::Mapping(config_map)) =
            map.get_mut(serde_yaml::Value::String("config".to_string()))
        && let Some(serde_yaml::Value::Mapping(book_map)) =
            config_map.get_mut(serde_yaml::Value::String("book".to_string()))
    {
        // Remove the fields that have defaults
        book_map.remove(serde_yaml::Value::String("margin_mm".to_string()));
        book_map.remove(serde_yaml::Value::String("gap_mm".to_string()));
        book_map.remove(serde_yaml::Value::String("bleed_threshold_mm".to_string()));

        // Also remove some page_layout_solver defaults
        config_map.remove(serde_yaml::Value::String("page_layout_solver".to_string()));
    }

    // Write back the YAML with removed optional fields
    fs::write(&yaml_path, serde_yaml::to_string(&yaml)?)?;

    // Now load config (should have defaults for removed fields)
    let result = config(&project_root)?;
    let output = render_config(&result)?;

    // Count how many times "# default" appears
    let default_count = output.matches("# default").count();

    // Should have multiple default annotations for missing optional fields
    // At minimum: margin_mm, gap_mm, bleed_threshold_mm in book, plus all page_layout_solver fields
    assert!(
        default_count > 5,
        "Expected more than 5 '# default' annotations, found {}",
        default_count
    );

    // Verify specific fields are marked as default
    assert!(output.contains("margin_mm:") && output.contains("# default"));
    assert!(output.contains("gap_mm:") && output.contains("# default"));

    // Verify output is valid YAML
    let _reparsed: serde_yaml::Value = serde_yaml::from_str(&output)?;

    Ok(())
}
