//! Integration tests for `fotobuch status` command

use anyhow::Result;
use fotobuch::commands::project::new::{NewConfig, project_new};
use fotobuch::commands::{AddConfig, add, build::*, status::*};
use fotobuch::dto_models::ProjectState;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test project
fn create_test_project(temp_dir: &TempDir) -> Result<PathBuf> {
    let config = NewConfig {
        name: "teststatus".to_string(),
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
    Ok(result.result.project_root)
}

/// Helper to create a project with photos and build layout
fn create_test_project_with_layout(temp_dir: &TempDir) -> Result<PathBuf> {
    let project_root = create_test_project(temp_dir)?;

    // Add test photos
    let photos_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("test_photos_unique");

    let add_config = AddConfig {
        source_filters: vec![],
        paths: vec![photos_path],
        allow_duplicates: false,
        xmp_filters: vec![],
        dry_run: false,
        update: false,
        recursive: true,
        weight: 1.0,
    };
    add(&project_root, &add_config)?;

    // Run initial build to create layout
    let build_config = BuildConfig {
        release: false,
        force: false,
        pages: None,
    };
    build(&project_root, &build_config)?;

    Ok(project_root)
}

#[test]
fn test_status_empty_layout() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    let config = StatusConfig { page: None };
    let report = status(&project_root, &config)?.result;

    assert_eq!(report.state, ProjectState_::Empty);
    assert_eq!(report.page_count, 0);
    assert_eq!(report.page_changes.len(), 0);
    assert!(report.detail.is_none());

    Ok(())
}

#[test]
fn test_status_clean() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    // Status should be clean after build
    let config = StatusConfig { page: None };
    let report = status(&project_root, &config)?.result;

    assert_eq!(report.state, ProjectState_::Clean);
    assert!(report.page_count > 0);
    assert_eq!(report.page_changes.len(), 0);
    assert!(report.detail.is_none());

    Ok(())
}

#[test]
fn test_status_with_unplaced() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    // Manually edit the YAML to move a photo from layout to unplaced
    let yaml_path = project_root.join("teststatus.yaml");
    let mut state = ProjectState::load(&yaml_path)?;

    if !state.layout.is_empty() && !state.layout[0].photos.is_empty() {
        // Remove a photo from layout but keep it in photos
        state.layout[0].photos.remove(0);
        state.save(&yaml_path)?;

        let config = StatusConfig { page: None };
        let report = status(&project_root, &config)?.result;

        // Should have unplaced photos
        assert!(report.unplaced > 0);
    }

    Ok(())
}

#[test]
fn test_status_page_detail() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let config = StatusConfig { page: Some(0) };
    let report = status(&project_root, &config)?.result;

    assert!(report.detail.is_some());
    let detail = report.detail.unwrap();
    assert_eq!(detail.page, 0);
    assert!(detail.photo_count > 0);
    assert!(!detail.slots.is_empty());

    // Check slot info structure
    for slot in &detail.slots {
        assert!(!slot.photo_id.is_empty());
        assert!(slot.ratio > 0.0);
        assert!(slot.swap_group.is_ascii_uppercase());
    }

    Ok(())
}

#[test]
fn test_status_page_detail_invalid_page() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let config = StatusConfig { page: Some(999) };
    let result = status(&project_root, &config);

    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(error.contains("Invalid page"));

    Ok(())
}

#[test]
fn test_status_counts_correct() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let config = StatusConfig { page: None };
    let report = status(&project_root, &config)?.result;

    // Verify basic counts are calculated
    assert!(report.total_photos > 0);
    assert!(report.group_count > 0);
    assert!(report.page_count > 0);
    assert!(report.avg_photos_per_page > 0.0);

    // Average should be roughly correct
    let expected_avg = report.total_photos as f64 / report.page_count as f64;
    assert!((report.avg_photos_per_page - expected_avg).abs() < 0.01);

    Ok(())
}

#[test]
fn test_status_swap_groups() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let config = StatusConfig { page: Some(0) };
    let report = status(&project_root, &config)?.result;

    if let Some(detail) = report.detail {
        // Check that swap groups are assigned
        let swap_groups: std::collections::HashSet<_> =
            detail.slots.iter().map(|s| s.swap_group).collect();

        // Should have at least one group
        assert!(!swap_groups.is_empty());

        // All should be uppercase letters
        for group in &swap_groups {
            assert!(group.is_ascii_uppercase());
        }
    }

    Ok(())
}

#[test]
fn test_status_consistency_no_orphans() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let config = StatusConfig { page: None };
    let report = status(&project_root, &config)?.result;

    // After normal build, should have no orphaned placements
    assert!(report.warnings.is_empty());

    Ok(())
}

#[test]
fn test_status_modified_after_manual_edit() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    // Manually edit the YAML to simulate a change
    let yaml_path = project_root.join("teststatus.yaml");
    let mut state = ProjectState::load(&yaml_path)?;

    // Modify the layout (e.g., remove a photo from a page)
    if !state.layout.is_empty() && !state.layout[0].photos.is_empty() {
        state.layout[0].photos.remove(0);
        state.save(&yaml_path)?;

        let config = StatusConfig { page: None };
        let report = status(&project_root, &config)?.result;

        // Status should show modifications
        assert_eq!(report.state, ProjectState_::Modified);
        assert!(!report.page_changes.is_empty());
    }

    Ok(())
}
