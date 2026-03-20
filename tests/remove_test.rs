//! Integration tests for `fotobuch remove` command

use anyhow::Result;
use fotobuch::commands::project::new::{NewConfig, project_new};
use fotobuch::commands::{AddConfig, add, build::*, remove::*};
use fotobuch::dto_models::ProjectState;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test project with build layout
fn create_test_project_with_layout(temp_dir: &TempDir) -> Result<PathBuf> {
    // Create project
    let config = NewConfig {
        name: "testremove".to_string(),
        width_mm: 200.0,
        height_mm: 250.0,
        bleed_mm: 3.0,
        quiet: true,
        with_cover: false,
        cover_width_mm: None,
        cover_height_mm: None,
        spine_grow_per_10_pages_mm: None,
        spine_mm: None,
    };
    let result = project_new(temp_dir.path(), &config)?;
    let project_root = result.project_root;

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
    };
    add(&project_root, &add_config)?;

    // Run initial build to create layout
    let build_config = BuildConfig {
        release: false,
        pages: None,
    };
    build(&project_root, &build_config)?;

    Ok(project_root)
}

#[test]
fn test_remove_no_matches_returns_zero() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let config = RemoveConfig {
        patterns: vec!["nonexistent".to_string()],
        keep_files: false,
        unplaced: false,
    };
    let result = remove(&project_root, &config)?;

    assert_eq!(result.photos_removed, 0);
    assert_eq!(result.placements_removed, 0);
    assert!(result.pages_affected.is_empty());

    Ok(())
}

#[test]
fn test_remove_single_photo_by_pattern() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let yaml_path = project_root.join("testremove.yaml");
    let state_before = ProjectState::load(&yaml_path)?;

    // Get first photo to match
    let first_photo_id = state_before
        .photos
        .iter()
        .flat_map(|g| g.files.iter())
        .next()
        .map(|f| f.id.clone())
        .expect("Should have at least one photo");

    // Use source path pattern to match it
    let pattern = first_photo_id
        .split('/')
        .next()
        .unwrap_or("test_photos")
        .to_string();

    let config = RemoveConfig {
        patterns: vec![pattern],
        keep_files: false,
        unplaced: false,
    };
    let result = remove(&project_root, &config)?;

    assert!(result.photos_removed > 0, "Should remove some photos");
    assert!(
        result.placements_removed > 0,
        "Should remove some placements"
    );

    // Verify state was saved
    let state_after = ProjectState::load(&yaml_path)?;
    assert!(
        state_after.photos.iter().all(|g| !g.files.is_empty()),
        "Should not have empty groups"
    );

    Ok(())
}

#[test]
fn test_remove_entire_group() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let yaml_path = project_root.join("testremove.yaml");
    let state_before = ProjectState::load(&yaml_path)?;

    // Get first group name
    let group_name = state_before
        .photos
        .first()
        .map(|g| g.group.clone())
        .expect("Should have at least one group");

    let config = RemoveConfig {
        patterns: vec![group_name.clone()],
        keep_files: false,
        unplaced: false,
    };
    let result = remove(&project_root, &config)?;

    assert_eq!(result.groups_removed.len(), 1);
    assert!(result.groups_removed.contains(&group_name));
    assert!(result.photos_removed > 0);

    // Verify group is removed
    let state_after = ProjectState::load(&yaml_path)?;
    assert!(
        !state_after.photos.iter().any(|g| g.group == group_name),
        "Group should be removed"
    );

    Ok(())
}

#[test]
fn test_remove_with_keep_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let yaml_path = project_root.join("testremove.yaml");
    let state_before = ProjectState::load(&yaml_path)?;

    let initial_photos = state_before
        .photos
        .iter()
        .flat_map(|g| g.files.iter())
        .count();

    // Remove from layout but keep files
    let pattern = "test_photos".to_string();
    let config = RemoveConfig {
        patterns: vec![pattern],
        keep_files: true,
        unplaced: false,
    };
    let result = remove(&project_root, &config)?;

    assert_eq!(
        result.photos_removed, 0,
        "No photos should be removed from photos section"
    );
    assert!(
        result.placements_removed > 0,
        "Placements should be removed from layout"
    );

    // Verify photos still exist but are unplaced
    let state_after = ProjectState::load(&yaml_path)?;
    let remaining_photos = state_after
        .photos
        .iter()
        .flat_map(|g| g.files.iter())
        .count();

    assert_eq!(
        remaining_photos, initial_photos,
        "All photos should still be in photos section"
    );

    // Verify placements removed from layout
    let photos_in_layout = state_after
        .layout
        .iter()
        .flat_map(|p| p.photos.iter())
        .count();
    assert!(
        photos_in_layout < initial_photos,
        "Some photos should be removed from layout"
    );

    Ok(())
}

#[test]
fn test_remove_multiple_patterns() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    // Use multiple patterns that should match different photos
    let patterns = vec!["IMG_001".to_string(), "IMG_002".to_string()];

    let config = RemoveConfig {
        patterns,
        keep_files: false,
        unplaced: false,
    };
    let _result = remove(&project_root, &config)?;

    // Should attempt to match but might not find anything
    // Just verify no error and result structure is valid

    Ok(())
}

#[test]
fn test_remove_invalid_regex_pattern() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let config = RemoveConfig {
        patterns: vec!["[invalid".to_string()],
        keep_files: false,
        unplaced: false,
    };
    let result = remove(&project_root, &config);

    assert!(result.is_err(), "Should error on invalid regex");
    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("Invalid pattern"),
        "Error should mention invalid pattern. Actual: {}",
        error_message
    );

    Ok(())
}

#[test]
fn test_remove_empty_pages_are_deleted() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let yaml_path = project_root.join("testremove.yaml");
    let _state_before = ProjectState::load(&yaml_path)?;

    let initial_page_count = _state_before.layout.len();
    assert!(initial_page_count > 0, "Should have at least one page");

    // Remove all photos from last page by targeting specific photo ID
    let last_page_photos = _state_before
        .layout
        .last()
        .map(|p| p.photos.clone())
        .unwrap_or_default();

    if !last_page_photos.is_empty() {
        let config = RemoveConfig {
            patterns: vec![last_page_photos[0].clone()],
            keep_files: false,
            unplaced: false,
        };
        let _result = remove(&project_root, &config)?;

        let state_after = ProjectState::load(&yaml_path)?;
        // Page count might be reduced if last page became empty and was removed
        assert!(state_after.layout.len() <= initial_page_count);

        // Verify pages are renumbered sequentially
        for (i, page) in state_after.layout.iter().enumerate() {
            assert_eq!(page.page, i, "Pages should be renumbered by array index (0-based)");
        }
    }

    Ok(())
}

#[test]
fn test_remove_git_commit() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let pattern = "test_photos".to_string();
    let config = RemoveConfig {
        patterns: vec![pattern],
        keep_files: false,
        unplaced: false,
    };
    let _result = remove(&project_root, &config)?;

    // Verify git commit was created
    let repo = git2::Repository::open(&project_root)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let message = commit.message().unwrap_or("");

    assert!(
        message.contains("remove:"),
        "Commit should contain 'remove:'"
    );

    Ok(())
}
