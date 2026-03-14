//! Integration tests for `fotobuch rebuild` command
mod common;

use anyhow::Result;
use photobook_solver::commands::build::{BuildConfig, build};
use photobook_solver::commands::project::new::{NewConfig, project_new};
use photobook_solver::commands::rebuild::{RebuildScope, rebuild};
use photobook_solver::commands::{AddConfig, add};
use photobook_solver::dto_models::ProjectState;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test project with photos and initial build
fn create_test_project_with_build(temp_dir: &TempDir) -> Result<PathBuf> {
    // Create project
    let config = NewConfig {
        name: "testrebuild".to_string(),
        width_mm: 200.0,
        height_mm: 250.0,
        bleed_mm: 3.0,
        quiet: true,
    };
    let result = project_new(temp_dir.path(), &config)?;
    let project_root = result.project_root;

    // Restrict book layout solver to force multiple pages with few photos
    let yaml_path = project_root.join("testrebuild.yaml");
    let mut state = ProjectState::load(&yaml_path)?;
    state.config.book_layout_solver.page_max = 5;
        state.config.book_layout_solver.page_target = 5;
    state.config.book_layout_solver.photos_per_page_max = 2; // Max 2 photos per page
    state.config.book_layout_solver.photos_per_page_min = 1; // Min 1 photo per page
    state.config.book_layout_solver.group_min_photos = 1; // Allow single-photo groups
    state.save(&yaml_path)?;

    // Add test photos (use only 5 photos for fast tests)
    let photos_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("test_artificial_photos_5");

    let add_config = AddConfig {
        paths: vec![photos_path],
        allow_duplicates: false,
        xmp_filter: None,
        source_filter: None,
        dry_run: false,
        update: false,
    };
    add(&project_root, &add_config)?;

    // Run initial build (with 5 photos and max 2 per page, we get at least 3 pages)
    let build_config = BuildConfig {
        release: false,
        pages: None,
    };
    build(&project_root, &build_config)?;

    Ok(project_root)
}

#[test]
fn test_rebuild_single_page_only_changes_slots() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_build(&temp_dir)?;

    let yaml_path = project_root.join("testrebuild.yaml");
    let state_before = ProjectState::load(&yaml_path)?;

    // Ensure we have at least 2 pages
    assert!(
        state_before.layout.len() >= 2,
        "Need at least 2 pages for test"
    );

    // Store state of all pages
    let page_to_rebuild = 1;
    let photos_before: Vec<_> = state_before
        .layout
        .iter()
        .map(|p| p.photos.clone())
        .collect();
    let slots_before: Vec<_> = state_before
        .layout
        .iter()
        .map(|p| p.slots.clone())
        .collect();

    // Rebuild single page
    let result = rebuild(&project_root, RebuildScope::SinglePage(page_to_rebuild))?;

    // Verify result
    assert_eq!(result.pages_rebuilt.len(), 1);
    assert_eq!(result.pages_rebuilt[0], page_to_rebuild);
    assert!(result.pdf_path.exists());

    // Load state after rebuild
    let state_after = ProjectState::load(&yaml_path)?;

    // Verify page count unchanged
    assert_eq!(state_after.layout.len(), state_before.layout.len());

    // Verify only the rebuilt page's slots changed, photos stay the same
    for (i, page) in state_after.layout.iter().enumerate() {
        let page_num = i + 1;

        // Photos should be identical for all pages
        assert_eq!(
            page.photos, photos_before[i],
            "Photos on page {} should not change",
            page_num
        );

        if page_num == page_to_rebuild {
            // Slots on rebuilt page may have changed (deterministic solver might give same result)
            // Just verify they exist
            assert!(!page.slots.is_empty(), "Rebuilt page should have slots");
        } else {
            // Other pages should be completely unchanged
            assert_eq!(
                page.slots, slots_before[i],
                "Slots on page {} should not change",
                page_num
            );
        }
    }

    // Verify git commit message
    let repo = git2::Repository::open(&project_root)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let message = commit.message().unwrap_or("");
    assert!(
        message.contains("rebuild:"),
        "Commit should mention 'rebuild'"
    );
    assert!(
        message.contains(&format!("page {}", page_to_rebuild)),
        "Commit should mention page number"
    );

    Ok(())
}

#[test]
fn test_rebuild_single_page_invalid_page_number() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_build(&temp_dir)?;

    let yaml_path = project_root.join("testrebuild.yaml");
    let state = ProjectState::load(&yaml_path)?;
    let page_count = state.layout.len();

    // Test page 0 (invalid)
    let result = rebuild(&project_root, RebuildScope::SinglePage(0));
    assert!(result.is_err(), "Page 0 should be invalid");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Invalid page"),
        "Error should mention invalid page"
    );

    // Test page > len (invalid)
    let result = rebuild(&project_root, RebuildScope::SinglePage(page_count + 1));
    assert!(result.is_err(), "Page beyond count should be invalid");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Invalid page"),
        "Error should mention invalid page"
    );

    Ok(())
}

#[test]
fn test_rebuild_range_replaces_pages() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_build(&temp_dir)?;

    let yaml_path = project_root.join("testrebuild.yaml");
    let state_before = ProjectState::load(&yaml_path)?;

    // Ensure we have at least 3 pages
    assert!(
        state_before.layout.len() >= 3,
        "Need at least 3 pages for test"
    );

    let start = 2;
    let end = 2; // Single page range

    // Store state of surrounding pages
    let page_before_range = state_before.layout[0].clone();
    let page_after_range = state_before.layout[2].clone();

    // Rebuild range with flex=0 (should keep same number of pages)
    let result = rebuild(
        &project_root,
        RebuildScope::Range {
            start,
            end,
            flex: 0,
        },
    )?;

    // Verify result
    assert!(!result.pages_rebuilt.is_empty());
    assert!(result.pdf_path.exists());

    // Load state after rebuild
    let state_after = ProjectState::load(&yaml_path)?;

    // With flex=0, page count should be the same
    assert_eq!(state_after.layout.len(), state_before.layout.len());

    // Verify surrounding pages unchanged
    assert_eq!(
        state_after.layout[0].photos, page_before_range.photos,
        "Page 1 (before range) should not change"
    );
    assert_eq!(
        state_after.layout[2].photos, page_after_range.photos,
        "Page 3 (after range) should not change"
    );

    // Verify git commit message
    let repo = git2::Repository::open(&project_root)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let message = commit.message().unwrap_or("");
    assert!(
        message.contains("rebuild:"),
        "Commit should mention 'rebuild'"
    );
    assert!(
        message.contains(&format!("pages {}-{}", start, end)),
        "Commit should mention page range"
    );

    Ok(())
}

#[test]
fn test_rebuild_range_flex_allows_page_variation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_build(&temp_dir)?;

    let yaml_path = project_root.join("testrebuild.yaml");
    let state_before = ProjectState::load(&yaml_path)?;

    // Ensure we have at least 3 pages
    assert!(
        state_before.layout.len() >= 3,
        "Need at least 3 pages for test"
    );

    let start = 1;
    let end = 2;
    let flex = 1;
    let original_range_size = end - start + 1;

    // Rebuild range with flex=2
    let result = rebuild(&project_root, RebuildScope::Range { start, end, flex })?;

    assert!(result.pdf_path.exists());

    // Load state after rebuild
    let state_after = ProjectState::load(&yaml_path)?;

    // Page count may vary
    let new_range_size = result.pages_rebuilt.len();
    assert!(
        new_range_size >= original_range_size.saturating_sub(flex),
        "New range size should be at least {} (original {} - flex {})",
        original_range_size.saturating_sub(flex),
        original_range_size,
        flex
    );
    assert!(
        new_range_size <= original_range_size + flex,
        "New range size should be at most {} (original {} + flex {})",
        original_range_size + flex,
        original_range_size,
        flex
    );

    // Verify pages are correctly renumbered (1-based, sequential)
    for (i, page) in state_after.layout.iter().enumerate() {
        assert_eq!(page.page, i + 1, "Page numbers should be sequential");
    }

    Ok(())
}

#[test]
fn test_rebuild_range_preserves_groups() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_build(&temp_dir)?;

    let yaml_path = project_root.join("testrebuild.yaml");
    let state_before = ProjectState::load(&yaml_path)?;

    // Ensure we have at least 3 pages
    assert!(
        state_before.layout.len() >= 3,
        "Need at least 3 pages for test"
    );

    let start = 1;
    let end = 2;

    // Collect photos that are in the range
    let photos_in_range: Vec<String> = state_before.layout[start - 1..end]
        .iter()
        .flat_map(|p| p.photos.iter().cloned())
        .collect();

    // For each photo, find its group
    let mut photo_to_group = std::collections::HashMap::new();
    for group in &state_before.photos {
        for file in &group.files {
            photo_to_group.insert(file.id.clone(), group.group.clone());
        }
    }

    // Rebuild range
    let result = rebuild(
        &project_root,
        RebuildScope::Range {
            start,
            end,
            flex: 0,
        },
    )?;

    // Load state after rebuild
    let state_after = ProjectState::load(&yaml_path)?;

    // With flex=0, the number of pages in the range should be the same
    let result_pages_len = result.pages_rebuilt.len();
    assert_eq!(
        result_pages_len,
        end - start + 1,
        "With flex=0, page count in range should remain the same"
    );

    // Verify the same photos are still in the new pages (possibly redistributed)
    let photos_after_rebuild: Vec<String> = state_after.layout
        [start - 1..start - 1 + result_pages_len]
        .iter()
        .flat_map(|p| p.photos.iter().cloned())
        .collect();

    // All original photos should still be present
    for photo_id in &photos_in_range {
        assert!(
            photos_after_rebuild.contains(photo_id),
            "Photo {} should still be in the rebuild range",
            photo_id
        );
    }

    Ok(())
}

#[test]
fn test_rebuild_all_redistributes_everything() -> Result<()> {
    common::init_tests();

    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_build(&temp_dir)?;

    let yaml_path = project_root.join("testrebuild.yaml");
    let state_before = ProjectState::load(&yaml_path)?;

    // Count total photos
    let total_photos: usize = state_before.photos.iter().map(|g| g.files.len()).sum();

    // Rebuild all
    eprintln!("About to call rebuild(All)...");
    let result = rebuild(&project_root, RebuildScope::All)?;
    eprintln!(
        "Rebuild returned: pages_rebuilt = {:?}, pdf exists = {}",
        result.pages_rebuilt,
        result.pdf_path.exists()
    );

    assert!(result.pdf_path.exists());
    assert!(!result.pages_rebuilt.is_empty());

    // Load state after rebuild
    let state_after = ProjectState::load(&yaml_path)?;

    // All photos from state.photos should be distributed
    let photos_in_layout: usize = state_after.layout.iter().map(|p| p.photos.len()).sum();

    assert_eq!(
        photos_in_layout, total_photos,
        "All photos should be distributed in layout"
    );

    // Page count may differ from before
    // (depends on solver's decision)
    assert!(
        !state_after.layout.is_empty(),
        "Should have at least one page"
    );

    // Verify pages are numbered correctly
    for (i, page) in state_after.layout.iter().enumerate() {
        assert_eq!(page.page, i + 1);
    }

    // Verify git commit message
    let repo = git2::Repository::open(&project_root)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let message = commit.message().unwrap_or("");
    eprintln!("Latest commit message: '{}'", message);

    // The rebuild should have created a new commit after the initial build
    // Check if this is a rebuild commit (not the initial build commit)
    if message.contains("build: initial layout") {
        // If HEAD is still the initial build, check the previous commit
        let parent = commit.parent(0)?;
        let parent_msg = parent.message().unwrap_or("");
        eprintln!("Parent commit message: '{}'", parent_msg);
        panic!(
            "Expected rebuild commit as HEAD, but found initial build commit. Parent: '{}'",
            parent_msg
        );
    }

    assert!(
        message.contains("rebuild"),
        "Commit should mention 'rebuild', but got: '{}'",
        message
    );

    Ok(())
}

#[test]
fn test_rebuild_without_layout_fails_except_all() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create project but don't build
    let config = NewConfig {
        name: "testrebuild".to_string(),
        width_mm: 200.0,
        height_mm: 250.0,
        bleed_mm: 3.0,
        quiet: true,
    };
    let result = project_new(temp_dir.path(), &config)?;
    let project_root = result.project_root;

    // Add test photos
    let photos_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("test_artificial_photos_5");

    let add_config = AddConfig {
        paths: vec![photos_path],
        allow_duplicates: false,
        xmp_filter: None,
        source_filter: None,
        dry_run: false,
        update: false,
    };
    add(&project_root, &add_config)?;

    // Verify no layout exists
    let yaml_path = project_root.join("testrebuild.yaml");
    let state = ProjectState::load(&yaml_path)?;
    assert!(state.layout.is_empty(), "Layout should be empty");

    // SinglePage should fail
    let result = rebuild(&project_root, RebuildScope::SinglePage(1));
    assert!(
        result.is_err(),
        "SinglePage rebuild without layout should fail"
    );
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("No layout exists"),
        "Error should mention missing layout"
    );

    // Range should fail
    let result = rebuild(
        &project_root,
        RebuildScope::Range {
            start: 1,
            end: 1,
            flex: 0,
        },
    );
    assert!(result.is_err(), "Range rebuild without layout should fail");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("No layout exists"),
        "Error should mention missing layout"
    );

    // All should succeed (like first build)
    let result = rebuild(&project_root, RebuildScope::All);
    assert!(result.is_ok(), "All rebuild should work without layout");

    let result = result?;
    assert!(result.pdf_path.exists());
    assert!(!result.pages_rebuilt.is_empty());

    // Verify layout was created
    let state_after = ProjectState::load(&yaml_path)?;
    assert!(!state_after.layout.is_empty(), "Layout should be created");

    Ok(())
}

#[test]
fn test_rebuild_range_invalid_range() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_build(&temp_dir)?;

    let yaml_path = project_root.join("testrebuild.yaml");
    let state = ProjectState::load(&yaml_path)?;
    let page_count = state.layout.len();

    // Test start=0
    let result = rebuild(
        &project_root,
        RebuildScope::Range {
            start: 0,
            end: 2,
            flex: 0,
        },
    );
    assert!(result.is_err(), "start=0 should be invalid");

    // Test end=0
    let result = rebuild(
        &project_root,
        RebuildScope::Range {
            start: 1,
            end: 0,
            flex: 0,
        },
    );
    assert!(result.is_err(), "end=0 should be invalid");

    // Test start > end
    let result = rebuild(
        &project_root,
        RebuildScope::Range {
            start: 3,
            end: 2,
            flex: 0,
        },
    );
    assert!(result.is_err(), "start > end should be invalid");

    // Test end > page_count
    let result = rebuild(
        &project_root,
        RebuildScope::Range {
            start: 1,
            end: page_count + 1,
            flex: 0,
        },
    );
    assert!(result.is_err(), "end beyond page count should be invalid");

    Ok(())
}
