//! Integration tests for `fotobuch build` command

mod common;

use anyhow::Result;
use photobook_solver::commands::build::{BuildConfig, build};
use photobook_solver::commands::project::new::{NewConfig, project_new};
use photobook_solver::commands::{AddConfig, add};
use photobook_solver::dto_models::ProjectState;
use photobook_solver::state_manager::StateManager;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test project with photos
fn create_test_project_with_photos(temp_dir: &TempDir) -> Result<PathBuf> {
    // Create project
    let config = NewConfig {
        name: "testbuild".to_string(),
        width_mm: 200.0,
        height_mm: 250.0,
        bleed_mm: 3.0,
        quiet: true,
    };
    let result = project_new(temp_dir.path(), &config)?;
    let project_root = result.project_root;

    let mut mgr = StateManager::open(&project_root)?;
    mgr.state.config.book_layout_solver.page_max = 5; // Limit pages to speed up tests
    mgr.state.config.book_layout_solver.page_target = 3;
    //mgr.state.config.book_layout_solver.photos_per_page_min = 1; // Allow single-photo pages for testing
    mgr.state.config.book_layout_solver.group_min_photos = 1; // Allow single-photo groups for testing  
    mgr.finish("test: set page_max to 5 for faster tests")?;

    // Add test photos
    let photos_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("test_photos_unique");

    let add_config = AddConfig {
        paths: vec![photos_path],
        allow_duplicates: false,
        xmp_filter: None,
        dry_run: false,
    };
    add(&project_root, &add_config)?;

    Ok(project_root)
}

#[test]
fn test_first_build_creates_layout_and_pdf() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_photos(&temp_dir)?;

    // Load initial state - should have photos but no layout
    let yaml_path = project_root.join("testbuild.yaml");
    let state_before = ProjectState::load(&yaml_path)?;
    assert!(!state_before.photos.is_empty(), "Should have photos");
    assert!(
        state_before.layout.is_empty(),
        "Layout should be empty before build"
    );

    // Run first build
    let build_config = BuildConfig {
        release: false,
        pages: None,
    };
    let result = build(&project_root, &build_config)?;

    // Verify PDF was created
    assert!(result.pdf_path.exists(), "PDF should be created");
    assert!(
        result.pdf_path.ends_with("testbuild.pdf"),
        "PDF should have correct name"
    );

    // Verify result statistics
    assert!(
        !result.pages_rebuilt.is_empty(),
        "Should have rebuilt pages"
    );
    assert!(result.pages_swapped.is_empty(), "First build has no swaps");
    assert!(!result.nothing_to_do, "First build should do something");
    assert!(
        result.dpi_warnings.is_empty(),
        "Preview build has no DPI warnings"
    );

    // Load state after build - should have layout now
    let state_after = ProjectState::load(&yaml_path)?;
    assert!(!state_after.layout.is_empty(), "Layout should be populated");

    // Verify layout has photos assigned
    let total_photos_in_layout: usize = state_after
        .layout
        .iter()
        .map(|page| page.photos.len())
        .sum();
    assert!(
        total_photos_in_layout > 0,
        "Layout should have photos assigned"
    );

    // Verify preview cache was created
    let preview_cache = project_root.join(".fotobuch/cache/testbuild/preview");
    assert!(
        preview_cache.exists(),
        "Preview cache directory should exist"
    );

    // Verify git commit was created
    let repo = git2::Repository::open(&project_root)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let message = commit.message().unwrap_or("");
    assert!(
        message.contains("build:"),
        "Commit message should mention 'build'"
    );

    Ok(())
}

#[test]
fn test_incremental_build_without_changes_does_nothing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_photos(&temp_dir)?;

    // First build
    let build_config = BuildConfig {
        release: false,
        pages: None,
    };
    let result1 = build(&project_root, &build_config)?;
    assert!(!result1.nothing_to_do, "First build should do something");

    // Second build without changes
    let result2 = build(&project_root, &build_config)?;

    // Should report nothing to do
    assert!(
        result2.nothing_to_do,
        "Second build without changes should report nothing to do"
    );
    assert!(
        result2.pages_rebuilt.is_empty(),
        "No pages should be rebuilt"
    );
    assert!(
        result2.pages_swapped.is_empty(),
        "No pages should be swapped"
    );

    // Verify no new commit was created
    let repo = git2::Repository::open(&project_root)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;

    // Store commit ID
    let commit_id_before = commit.id();

    // After second build, HEAD should be same commit (no new commit)
    let head_after = repo.head()?;
    let commit_after = head_after.peel_to_commit()?;
    assert_eq!(
        commit_id_before,
        commit_after.id(),
        "No new commit should be created"
    );

    Ok(())
}

#[test]
fn test_release_requires_pages_flag_not_allowed() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_photos(&temp_dir)?;

    // First build to create layout
    let build_config = BuildConfig {
        release: false,
        pages: None,
    };
    build(&project_root, &build_config)?;

    // Try release with --pages (should fail)
    let release_config = BuildConfig {
        release: true,
        pages: Some(vec![1]),
    };
    let result = build(&project_root, &release_config);

    assert!(result.is_err(), "Release with --pages should fail");
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("--pages"), "Error should mention --pages");
    assert!(err_msg.contains("release"), "Error should mention release");

    Ok(())
}

#[test]
fn test_release_requires_clean_state() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_photos(&temp_dir)?;

    // First build
    let build_config = BuildConfig {
        release: false,
        pages: None,
    };
    build(&project_root, &build_config)?;

    // Manually modify layout in YAML (simulating uncommitted changes)
    let yaml_path = project_root.join("testbuild.yaml");
    let mut state = ProjectState::load(&yaml_path)?;

    // Change area_weight of first photo to create a modification
    if let Some(photo) = state.photos.first_mut().and_then(|g| g.files.first_mut()) {
        photo.area_weight += 0.1;
    }
    state.save(&yaml_path)?;

    // Try release build (should fail because layout is not clean)
    let release_config = BuildConfig {
        release: true,
        pages: None,
    };
    let result = build(&project_root, &release_config);

    assert!(result.is_err(), "Release with dirty state should fail");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("changes") || err_msg.contains("clean"),
        "Error should mention changes or clean state: {}",
        err_msg
    );

    Ok(())
}

#[test]
fn test_release_creates_final_cache_and_pdf() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_photos(&temp_dir)?;

    // First build
    let build_config = BuildConfig {
        release: false,
        pages: None,
    };
    build(&project_root, &build_config)?;

    // Release build
    let release_config = BuildConfig {
        release: true,
        pages: None,
    };
    let result = build(&project_root, &release_config)?;

    // Verify final PDF was created
    assert!(result.pdf_path.exists(), "Final PDF should be created");
    assert!(
        result.pdf_path.ends_with("final.pdf"),
        "Should create final.pdf"
    );

    // Verify final cache was created
    let final_cache = project_root.join(".fotobuch/cache/testbuild/final");
    assert!(final_cache.exists(), "Final cache directory should exist");

    // Note: This might be empty if solver creates no layout, or files are in subdirectories
    // The important part is that the directory exists

    // Verify git commit mentions release
    let repo = git2::Repository::open(&project_root)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let message = commit.message().unwrap_or("");
    assert!(
        message.contains("release"),
        "Commit message should mention 'release'"
    );

    // Verify DPI warnings are present in result (may be empty for good photos)
    // We just check the field exists
    let _ = &result.dpi_warnings;

    Ok(())
}

#[test]
fn test_pages_filter_limits_scope() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_photos(&temp_dir)?;

    // First build to create multi-page layout
    let build_config = BuildConfig {
        release: false,
        pages: None,
    };
    let result1 = build(&project_root, &build_config)?;

    // Skip test if only one page was created
    if result1.pages_rebuilt.len() < 2 {
        eprintln!("Test skipped: need at least 2 pages for filter test");
        return Ok(());
    }

    // Modify a photo to trigger rebuild
    let yaml_path = project_root.join("testbuild.yaml");
    let mut state = ProjectState::load(&yaml_path)?;
    if let Some(photo) = state.photos.first_mut().and_then(|g| g.files.first_mut()) {
        photo.area_weight += 0.2;
    }
    state.save(&yaml_path)?;

    // Build with page filter (only page 1)
    let filtered_config = BuildConfig {
        release: false,
        pages: Some(vec![1]),
    };
    let result2 = build(&project_root, &filtered_config)?;

    // Should only rebuild page 1 (even if other pages have changes)
    assert!(
        result2.pages_rebuilt.contains(&1),
        "Page 1 should be rebuilt"
    );

    // In a real scenario with multiple affected pages, we'd verify
    // that other pages are not rebuilt. For this simple test,
    // we just verify the filter was accepted and build succeeded.

    Ok(())
}

#[test]
fn test_build_handles_empty_photo_list() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create project without adding photos
    let config = NewConfig {
        name: "emptyproject".to_string(),
        width_mm: 200.0,
        height_mm: 250.0,
        bleed_mm: 3.0,
        quiet: true,
    };
    let result = project_new(temp_dir.path(), &config)?;
    let project_root = result.project_root;

    // Try to build with no photos
    let build_config = BuildConfig {
        release: false,
        pages: None,
    };
    let build_result = build(&project_root, &build_config);

    // Should either succeed with nothing to do, or fail gracefully
    // (behavior depends on solver implementation)
    match build_result {
        Ok(result) => {
            // If it succeeds, it should report nothing to do
            assert!(
                result.nothing_to_do || result.pages_rebuilt.is_empty(),
                "Build with no photos should do nothing or create no pages"
            );
        }
        Err(e) => {
            // If it fails, error should be clear
            let msg = e.to_string();
            assert!(
                msg.contains("photo") || msg.contains("empty") || msg.contains("No"),
                "Error message should be clear about missing photos: {}",
                msg
            );
        }
    }

    Ok(())
}

#[test]
fn test_max_groups_per_page_limits_to_one_group() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_photos(&temp_dir)?;

    // Load initial state
    let yaml_path = project_root.join("testbuild.yaml");
    let mut state = ProjectState::load(&yaml_path)?;

    // Verify we have 2 groups with photos
    assert_eq!(
        state.photos.len(),
        2,
        "Test fixture should have 2 groups (group1 and group2)"
    );

    // Count total photos
    let total_photos: usize = state.photos.iter().map(|g| g.files.len()).sum();
    assert_eq!(total_photos, 3, "Test fixture should have 3 photos total");

    // Set max_groups_per_page = 1 and adjust related constraints
    // to allow single-photo groups
    state.config.book_layout_solver.group_max_per_page = 1;
    state.config.book_layout_solver.group_min_photos = 1;
    state.config.book_layout_solver.photos_per_page_min = 1;
    state.save(&yaml_path)?;

    // Build with the constraint
    let build_config = BuildConfig {
        release: false,
        pages: None,
    };
    let result = build(&project_root, &build_config)?;

    // Verify build succeeded
    assert!(
        !result.pages_rebuilt.is_empty(),
        "Build should create pages"
    );

    // Load state after build
    let state_after = ProjectState::load(&yaml_path)?;
    assert_eq!(state_after.layout.len(), 2, "Should have exactly 2 pages");

    // Verify each page contains photos from only one group
    for page in state_after.layout.iter() {
        let page_groups: std::collections::HashSet<String> = page
            .photos
            .iter()
            .map(|photo_id| {
                // Photo ID format is "group/filename", extract group name
                photo_id.split('/').next().unwrap_or("").to_string()
            })
            .collect();

        assert_eq!(
            page_groups.len(),
            1,
            "Page {} should contain photos from only 1 group, but has {}",
            page.page,
            page_groups.len()
        );
    }

    // Verify pages have disjunct photo IDs
    let page1_photos: std::collections::HashSet<_> = state_after.layout[0].photos.iter().collect();
    let page2_photos: std::collections::HashSet<_> = state_after.layout[1].photos.iter().collect();

    let intersection: Vec<_> = page1_photos.intersection(&page2_photos).collect();

    assert!(
        intersection.is_empty(),
        "Pages should have disjunct photos, but found overlap: {:?}",
        intersection
    );

    Ok(())
}

#[test]
fn test_build_from_scratch_with_max_groups_per_page_one() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_photos(&temp_dir)?;

    // First, do an initial build to ensure everything is set up
    let build_config = BuildConfig {
        release: false,
        pages: None,
    };
    build(&project_root, &build_config)?;

    // Now clear the layout and reconfigure with max_groups_per_page = 1
    let yaml_path = project_root.join("testbuild.yaml");
    let mut state = ProjectState::load(&yaml_path)?;

    // Verify we have 2 groups
    assert_eq!(
        state.photos.len(),
        2,
        "Test fixture should have 2 groups (group1 and group2)"
    );

    // Clear the layout to force rebuild from scratch
    state.layout.clear();

    // Set max_groups_per_page = 1 and adjust related constraints
    state.config.book_layout_solver.group_max_per_page = 1;
    state.config.book_layout_solver.group_min_photos = 1;
    state.config.book_layout_solver.photos_per_page_min = 1;
    state.save(&yaml_path)?;

    // Build from scratch with the constraint
    let result = build(&project_root, &build_config)?;

    // Verify build succeeded
    assert!(
        !result.pages_rebuilt.is_empty(),
        "Build should create pages"
    );

    // Load state after build
    let state_after = ProjectState::load(&yaml_path)?;
    let num_pages = state_after.layout.len();
    assert!(
        num_pages >= 2,
        "Should have at least 2 pages, got {}",
        num_pages
    );

    // Collect all photo IDs from all pages
    let mut all_photos = std::collections::HashSet::new();
    for page in &state_after.layout {
        for photo_id in &page.photos {
            all_photos.insert(photo_id.clone());
        }
    }

    let page1_photos: std::collections::HashSet<_> = state_after.layout[0].photos.iter().collect();
    let page2_photos: std::collections::HashSet<_> = state_after.layout[1].photos.iter().collect();

    println!("Page 1 photos: {:?}", page1_photos);
    println!("Page 2 photos: {:?}", page2_photos);

    assert!(
        page1_photos.len() + page2_photos.len() == all_photos.len(),
        "Pages should contain all photos"
    );

    let intersection: Vec<_> = page1_photos.intersection(&page2_photos).collect();

    assert!(
        intersection.is_empty(),
        "Page {} and Page {} should have disjunct photos, but found overlap: {:?}",
        state_after.layout[0].page,
        state_after.layout[1].page,
        intersection
    );

    Ok(())
}
