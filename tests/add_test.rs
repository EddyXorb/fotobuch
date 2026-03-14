//! Integration tests for `fotobuch add` command

use anyhow::Result;
use photobook_solver::commands::project::new::{NewConfig, project_new};
use photobook_solver::commands::{AddConfig, add};
use photobook_solver::dto_models::ProjectState;
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper to create a test project
fn create_test_project(temp_dir: &TempDir) -> Result<PathBuf> {
    let config = NewConfig {
        name: "testproject".to_string(),
        width_mm: 200.0,
        height_mm: 250.0,
        bleed_mm: 3.0,
        quiet: true,
    };
    let result = project_new(temp_dir.path(), &config)?;
    Ok(result.project_root)
}

/// Helper to get absolute path to test fixtures
fn test_photos_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("test_photos_unique")
}

#[test]
fn test_add_single_directory_creates_groups() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    // Add photos from test fixtures
    let add_config = AddConfig {
        paths: vec![test_photos_path()],
        allow_duplicates: false,
        xmp_filter: None,
        source_filter: None,
        dry_run: false,
        update: false,
    };

    let result = add(&project_root, &add_config)?;

    // Verify result statistics
    assert!(
        !result.groups_added.is_empty(),
        "Should have added at least one group"
    );
    assert_eq!(result.skipped, 0, "No duplicates on first add");
    assert_eq!(result.warnings.len(), 0, "No warnings on first add");

    // Load YAML and verify photos were added
    let yaml_path = project_root.join("testproject.yaml");
    let state = ProjectState::load(&yaml_path)?;

    assert!(!state.photos.is_empty(), "Photos should be in YAML");

    // Verify each photo has required fields
    for group in &state.photos {
        assert!(!group.group.is_empty(), "Group name should be set");
        assert!(!group.sort_key.is_empty(), "Sort key should be set");

        for photo in &group.files {
            assert!(!photo.id.is_empty(), "Photo ID should be set");
            assert!(!photo.source.is_empty(), "Photo source should be set");
            assert!(
                !photo.hash.is_empty(),
                "Photo hash should be persisted to YAML"
            );
            assert_eq!(
                photo.hash.len(),
                64,
                "Blake3 hash should be 64 hex characters"
            );
            assert!(photo.width_px > 0, "Photo width should be set");
            assert!(photo.height_px > 0, "Photo height should be set");
        }
    }

    // Verify git commit was created
    let repo = git2::Repository::open(&project_root)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let message = commit.message().unwrap_or("");
    assert!(
        message.contains("add:"),
        "Commit message should mention 'add'"
    );

    Ok(())
}

#[test]
fn test_add_duplicate_path_skips() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    let add_config = AddConfig {
        paths: vec![test_photos_path()],
        allow_duplicates: false,
        xmp_filter: None,
        source_filter: None,
        dry_run: false,
        update: false,
    };

    // First add
    let result1 = add(&project_root, &add_config)?;
    let photos_added_first = result1
        .groups_added
        .iter()
        .map(|g| g.photo_count)
        .sum::<usize>();

    // Second add (should skip all)
    let result2 = add(&project_root, &add_config)?;

    assert_eq!(
        result2.groups_added.len(),
        0,
        "No new groups should be added"
    );
    assert_eq!(
        result2.skipped, photos_added_first,
        "All photos should be skipped"
    );
    assert_eq!(result2.warnings.len(), 0, "No warnings for path duplicates");

    Ok(())
}

#[test]
fn test_add_merges_existing_group() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    // Add group1 only
    let group1_path = test_photos_path().join("group1");
    let add_config1 = AddConfig {
        paths: vec![group1_path.clone()],
        allow_duplicates: false,
        xmp_filter: None,
        source_filter: None,
        dry_run: false,
        update: false,
    };
    let result1 = add(&project_root, &add_config1)?;

    let yaml_path = project_root.join("testproject.yaml");
    let state1 = ProjectState::load(&yaml_path)?;
    let initial_group_count = state1.photos.len();
    let initial_photo_count: usize = state1.photos.iter().map(|g| g.files.len()).sum();

    // Add group2
    let group2_path = test_photos_path().join("group2");
    let add_config2 = AddConfig {
        paths: vec![group2_path],
        allow_duplicates: false,
        xmp_filter: None,
        source_filter: None,
        dry_run: false,
        update: false,
    };
    let _result2 = add(&project_root, &add_config2)?;

    let state2 = ProjectState::load(&yaml_path)?;
    let final_group_count = state2.photos.len();
    let final_photo_count: usize = state2.photos.iter().map(|g| g.files.len()).sum();

    assert!(
        final_group_count > initial_group_count,
        "Should have added a new group"
    );
    assert!(
        final_photo_count > initial_photo_count,
        "Should have added new photos"
    );

    // Add group1 again (should merge with existing)
    let result3 = add(&project_root, &add_config1)?;
    assert_eq!(
        result3.skipped,
        result1
            .groups_added
            .iter()
            .map(|g| g.photo_count)
            .sum::<usize>(),
        "Should skip all photos from group1 as they already exist"
    );

    Ok(())
}

#[test]
fn test_add_allow_duplicates_flag() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    // Create a temporary directory structure with duplicate files
    let temp_photo_dir1 = temp_dir.path().join("photos1");
    let temp_photo_dir2 = temp_dir.path().join("photos2");
    fs::create_dir_all(&temp_photo_dir1)?;
    fs::create_dir_all(&temp_photo_dir2)?;

    // Source file
    let source_file = test_photos_path().join("group1/photo1.jpg");

    // Create copies in different directories
    let copy1 = temp_photo_dir1.join("photo.jpg");
    let copy2 = temp_photo_dir2.join("photo.jpg");

    fs::copy(&source_file, &copy1)?;
    fs::copy(&source_file, &copy2)?;

    // Add first directory
    let add_config1 = AddConfig {
        paths: vec![temp_photo_dir1.clone()],
        allow_duplicates: false,
        xmp_filter: None,
        source_filter: None,
        dry_run: false,
        update: false,
    };
    let result1 = add(&project_root, &add_config1)?;
    assert_eq!(
        result1
            .groups_added
            .iter()
            .map(|g| g.photo_count)
            .sum::<usize>(),
        1
    );

    // Try to add second directory (same hash) without allow_duplicates
    let add_config2 = AddConfig {
        paths: vec![temp_photo_dir2.clone()],
        allow_duplicates: false,
        xmp_filter: None,
        source_filter: None,
        dry_run: false,
        update: false,
    };
    let result2 = add(&project_root, &add_config2)?;

    assert_eq!(result2.skipped, 1, "Should skip hash duplicate");
    assert!(
        !result2.warnings.is_empty(),
        "Should warn about hash duplicate"
    );
    assert!(
        result2.warnings[0].contains("Duplicate"),
        "Warning should mention duplicate"
    );

    // Now add second directory with allow_duplicates = true
    let add_config3 = AddConfig {
        paths: vec![temp_photo_dir2],
        allow_duplicates: true,
        xmp_filter: None,
        source_filter: None,
        dry_run: false,
        update: false,
    };
    let result3 = add(&project_root, &add_config3)?;
    assert_eq!(
        result3
            .groups_added
            .iter()
            .map(|g| g.photo_count)
            .sum::<usize>(),
        1,
        "Should add the duplicate when allowed"
    );
    assert_eq!(
        result3.warnings.len(),
        0,
        "No warnings when duplicates allowed"
    );

    Ok(())
}

#[test]
fn test_add_sorts_groups_by_sort_key() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    // Add all photos
    let add_config = AddConfig {
        paths: vec![test_photos_path()],
        allow_duplicates: false,
        xmp_filter: None,
        source_filter: None,
        dry_run: false,
        update: false,
    };
    add(&project_root, &add_config)?;

    // Load YAML and verify groups are sorted
    let yaml_path = project_root.join("testproject.yaml");
    let state = ProjectState::load(&yaml_path)?;

    assert!(!state.photos.is_empty(), "Should have photos");

    // Verify sort order
    for i in 1..state.photos.len() {
        assert!(
            state.photos[i - 1].sort_key <= state.photos[i].sort_key,
            "Groups should be sorted by sort_key"
        );
    }

    Ok(())
}

#[test]
fn test_dry_run_does_not_write_state() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    let yaml_path = project_root.join("testproject.yaml");
    let state_before = ProjectState::load(&yaml_path)?;

    let add_config = AddConfig {
        paths: vec![test_photos_path()],
        allow_duplicates: false,
        xmp_filter: None,
        source_filter: None,
        dry_run: true,
        update: false,
    };
    let result = add(&project_root, &add_config)?;

    assert!(result.dry_run);
    assert!(
        !result.groups_added.is_empty(),
        "Dry run should still report what would be added"
    );

    let state_after = ProjectState::load(&yaml_path)?;
    assert_eq!(
        state_before.photos.len(),
        state_after.photos.len(),
        "Dry run must not modify the persisted state"
    );

    Ok(())
}

#[test]
fn test_xmp_filter_with_no_match_excludes_nothing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    // Pattern that matches nothing in XMP metadata of plain test fixtures
    let add_config = AddConfig {
        paths: vec![test_photos_path()],
        allow_duplicates: false,
        xmp_filter: Some(Regex::new(r"THIS_WILL_NEVER_MATCH_ANY_XMP_abc123xyz").unwrap()),
        source_filter: None,
        dry_run: true,
        update: false,
    };
    let result = add(&project_root, &add_config)?;

    assert_eq!(
        result.groups_added.len(),
        2,
        "All photos should not be excluded when XMP filter matches nothing"
    );

    Ok(())
}

#[test]
fn test_xmp_filter_matches_modified_description() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    // Copy test_photos_unique into temp directory
    let temp_photos = temp_dir.path().join("test_photos");
    fs::create_dir_all(&temp_photos)?;

    let source_photos = test_photos_path();
    copy_dir_recursive(&source_photos, &temp_photos)?;

    // Use exiftool to set XMP description field
    Command::new("exiftool")
        .arg("-XMP:Description=LEER")
        .arg("-overwrite_original")
        .arg(temp_photos.join("group1/photo1.jpg"))
        .arg(temp_photos.join("group2/photo3.jpg"))
        .output()
        .expect("Failed to execute exiftool");

    // Manipulate one photo (photo2.jpg from group2) using exiftool to set XMP description
    let photo_to_match = temp_photos.join("group2/photo2.jpg");

    // Use exiftool to set XMP description field
    let output = Command::new("exiftool")
        .arg("-XMP:Description=HIER_STEHT_WAS")
        .arg("-overwrite_original")
        .arg(&photo_to_match)
        .output()
        .expect("Failed to execute exiftool");

    assert!(
        output.status.success(),
        "exiftool failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Add photos with XMP filter that matches ".*STEHT.*"
    let add_config = AddConfig {
        paths: vec![temp_photos],
        allow_duplicates: false,
        xmp_filter: Some(Regex::new(r".*STEHT.*")?),
        source_filter: None,
        dry_run: false,
        update: false,
    };
    let result = add(&project_root, &add_config)?;

    // Verify that exactly 2 photos were filtered out (xmp_filtered = 2)
    assert_eq!(
        result.xmp_filtered, 2,
        "Should have filtered out 2 photos without matching XMP"
    );

    // Verify that exactly 1 group was added with 1 photo
    assert_eq!(
        result.groups_added.len(),
        1,
        "Should have added exactly 1 group"
    );
    assert_eq!(
        result.groups_added[0].photo_count, 1,
        "Group should contain exactly 1 photo"
    );

    // Load YAML and verify only the modified photo was added
    let yaml_path = project_root.join("testproject.yaml");
    let state = ProjectState::load(&yaml_path)?;

    assert_eq!(
        state.photos.len(),
        1,
        "Should have exactly 1 photo group in YAML"
    );
    assert_eq!(
        state.photos[0].files.len(),
        1,
        "Group should have exactly 1 photo file"
    );

    // Verify the added photo is the one we modified (photo2.jpg)
    let added_photo = &state.photos[0].files[0];
    assert!(
        added_photo.source.contains("photo2.jpg"),
        "Added photo should be photo2.jpg, got: {}",
        added_photo.source
    );

    Ok(())
}

/// Helper function to recursively copy directories
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dest_path = dst.join(&file_name);

        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
        }
    }

    Ok(())
}

#[test]
fn test_add_handles_missing_directory() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    let nonexistent_path = temp_dir.path().join("nonexistent");
    let add_config = AddConfig {
        paths: vec![nonexistent_path],
        allow_duplicates: false,
        xmp_filter: None,
        source_filter: None,
        dry_run: false,
        update: false,
    };

    // Should return an error for missing directory
    let result = add(&project_root, &add_config);
    assert!(result.is_err(), "Should fail when directory doesn't exist");

    Ok(())
}

#[test]
fn test_add_hashes_are_persisted() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project(&temp_dir)?;

    let add_config = AddConfig {
        paths: vec![test_photos_path()],
        allow_duplicates: false,
        xmp_filter: None,
        source_filter: None,
        dry_run: false,
        update: false,
    };
    let result = add(&project_root, &add_config)?;

    let yaml_path = project_root.join("testproject.yaml");
    let state = ProjectState::load(&yaml_path)?;

    // Verify all photos are persisted
    let total_photos: usize = state.photos.iter().map(|g| g.files.len()).sum();
    let expected_photos: usize = result.groups_added.iter().map(|g| g.photo_count).sum();

    assert_eq!(
        total_photos, expected_photos,
        "All added photos should be in YAML"
    );

    // Verify hashes are persisted to YAML
    for group in &state.photos {
        for photo in &group.files {
            assert!(!photo.id.is_empty(), "Photo ID should be set");
            assert!(!photo.source.is_empty(), "Photo source should be set");
            assert!(!photo.hash.is_empty(), "Hash should be persisted to YAML");
            assert_eq!(
                photo.hash.len(),
                64,
                "Blake3 hash should be 64 hex characters"
            );
            assert!(
                photo.hash.chars().all(|c| c.is_ascii_hexdigit()),
                "Hash should be hexadecimal"
            );
        }
    }

    Ok(())
}
