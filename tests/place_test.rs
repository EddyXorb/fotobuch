//! Integration tests for `fotobuch place` command

use anyhow::Result;
use fotobuch::commands::project::new::{NewConfig, project_new};
use fotobuch::commands::{AddConfig, add, build::*, place::*};
use fotobuch::dto_models::ProjectState;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test project with build layout
fn create_test_project_with_layout(temp_dir: &TempDir) -> Result<PathBuf> {
    // Create project
    let config = NewConfig {
        name: "testplace".to_string(),
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
        base_config: None,
    };
    let result = project_new(temp_dir.path(), &config)?;
    let project_root = result.result.project_root;

    // Configure solver for fast tests
    let yaml_path = project_root.join("testplace.yaml");
    let mut state = ProjectState::load(&yaml_path)?;
    state.config.book_layout_solver.page_max = 5;
    state.config.book_layout_solver.page_target = 3;
    state.config.book_layout_solver.enable_local_search = false;
    state.config.page_layout_solver.population_size = 20;
    state.config.page_layout_solver.max_generations = 10;
    state.config.page_layout_solver.islands_nr = 1;
    state.save(&yaml_path)?;

    // Add test photos
    let photos_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("test_photos_unique");

    let add_config = AddConfig {
        paths: vec![photos_path],
        allow_duplicates: false,
        xmp_filters: vec![],
        source_filters: vec![],
        dry_run: false,
        update: false,
        recursive: true,
        weight: 1.0,
    };
    add(&project_root, &add_config)?;

    // Run initial build to create layout
    let build_config = BuildConfig {
        plan: BuildPlan::Auto { pages: None },
        skip_pdf: true,
        skip_cache_update: true,
    };
    build(&project_root, &build_config)?;

    Ok(project_root)
}

#[test]
fn test_place_no_unplaced_photos_returns_zero() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    // All photos are placed from build - place should do nothing
    let config = PlaceConfig {
        filters: vec![],
        ids: vec![],
        dst: PlaceDst::Auto,
    };
    let result = place(&project_root, &config)?;

    assert_eq!(result.result.photos_placed, 0);
    assert!(result.result.pages_affected.is_empty());

    Ok(())
}

#[test]
fn test_place_requires_layout() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create project without build (no layout)
    let config = NewConfig {
        name: "nobuildbuild".to_string(),
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
        base_config: None,
    };
    let result = project_new(temp_dir.path(), &config)?;
    let project_root = result.result.project_root;

    // Add photos
    let photos_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("test_photos_unique");

    let add_config = AddConfig {
        paths: vec![photos_path],
        allow_duplicates: false,
        xmp_filters: vec![],
        source_filters: vec![],
        dry_run: false,
        update: false,
        recursive: true,
        weight: 1.0,
    };
    add(&project_root, &add_config)?;

    // Try to place without build - should fail
    let config = PlaceConfig {
        filters: vec![],
        ids: vec![],
        dst: PlaceDst::Auto,
    };
    let result = place(&project_root, &config);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No layout yet"));

    Ok(())
}

#[test]
fn test_place_with_invalid_page_number() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    // Load state to see how many pages exist
    let yaml_path = project_root.join("testplace.yaml");
    let state = ProjectState::load(&yaml_path)?;
    let page_count = state.layout.len();
    assert!(page_count > 0);

    // Try placing into page beyond layout (0-based, so page_count is out of bounds)
    let config = PlaceConfig {
        filters: vec![],
        ids: vec![],
        dst: PlaceDst::Page(page_count),
    };
    let result = place(&project_root, &config);
    assert!(result.is_err());

    // Try placing far beyond layout
    let config = PlaceConfig {
        filters: vec![],
        ids: vec![],
        dst: PlaceDst::Page(page_count + 10),
    };
    let result = place(&project_root, &config);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_place_into_specific_page() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    // Load initial state
    let yaml_path = project_root.join("testplace.yaml");
    let state_before = ProjectState::load(&yaml_path)?;

    // Remove first photo and first slot from first page to create an unplaced photo
    let mut state_modified = state_before.clone();
    let removed_photo =
        if !state_modified.layout.is_empty() && !state_modified.layout[0].photos.is_empty() {
            state_modified.layout[0].photos.remove(0)
        } else {
            return Ok(()); // Skip test if no photos
        };
    if !state_modified.layout[0].slots.is_empty() {
        state_modified.layout[0].slots.remove(0);
    }
    state_modified.save(&yaml_path)?;

    // Now place the unplaced photo into page 0 (0-based first page)
    let config = PlaceConfig {
        filters: vec![],
        ids: vec![],
        dst: PlaceDst::Page(0),
    };
    let result = place(&project_root, &config)?;

    assert_eq!(
        result.result.photos_placed, 1,
        "Should place exactly 1 photo"
    );
    assert_eq!(
        result.result.pages_affected,
        vec![0],
        "Only page 0 should be affected"
    );

    // Verify state was saved and photo is back on page 0
    let state_after = ProjectState::load(&yaml_path)?;
    assert!(
        state_after.layout[0].photos.contains(&removed_photo),
        "Removed photo should be on page 0"
    );

    // Verify git commit
    let repo = git2::Repository::open(&project_root)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let message = commit.message().unwrap_or("");
    assert!(message.contains("place:"), "Commit should mention place");
    assert!(
        message.contains("page 0"),
        "Commit should mention page number"
    );

    Ok(())
}

#[test]
fn test_place_filter_by_pattern() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let yaml_path = project_root.join("testplace.yaml");
    let state_before = ProjectState::load(&yaml_path)?;

    // Remove first photo and slot from first page to create an unplaced photo
    let mut state_modified = state_before.clone();
    if !state_modified.layout.is_empty() && !state_modified.layout[0].photos.is_empty() {
        state_modified.layout[0].photos.remove(0);
        if !state_modified.layout[0].slots.is_empty() {
            state_modified.layout[0].slots.remove(0);
        }
    }
    state_modified.save(&yaml_path)?;

    // Place with a pattern that matches the test fixture path
    let config = PlaceConfig {
        filters: vec!["test_photos".to_string()],
        ids: vec![],
        dst: PlaceDst::Auto,
    };
    let result = place(&project_root, &config)?;

    // Should place at least the one unplaced photo if it matches pattern
    assert!(
        result.result.photos_placed > 0,
        "Should place some photos matching pattern"
    );

    Ok(())
}

#[test]
fn test_place_chronologically_without_unplaced() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    // All photos already placed from build
    let config = PlaceConfig {
        filters: vec![],
        ids: vec![],
        dst: PlaceDst::Auto,
    };
    let result = place(&project_root, &config)?;

    assert_eq!(result.result.photos_placed, 0);
    assert!(result.result.pages_affected.is_empty());

    Ok(())
}

#[test]
fn test_place_invalid_filter_pattern() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let yaml_path = project_root.join("testplace.yaml");
    let state_before = ProjectState::load(&yaml_path)?;

    // Remove first photo and slot to create an unplaced photo
    let mut state_modified = state_before;
    if !state_modified.layout.is_empty() && !state_modified.layout[0].photos.is_empty() {
        state_modified.layout[0].photos.remove(0);
        if !state_modified.layout[0].slots.is_empty() {
            state_modified.layout[0].slots.remove(0);
        }
    }
    state_modified.save(&yaml_path)?;

    // Use invalid regex pattern
    let config = PlaceConfig {
        filters: vec!["[invalid".to_string()],
        ids: vec![],
        dst: PlaceDst::Auto,
    };
    let result = place(&project_root, &config);

    assert!(result.is_err());
    let error_message = result.unwrap_err().to_string();

    assert!(
        error_message.contains("Invalid filter pattern"),
        "Actual: {}",
        &error_message
    );

    Ok(())
}

#[test]
fn test_place_ids_filter_restricts_to_selected_photos() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let yaml_path = project_root.join("testplace.yaml");
    let mut state = ProjectState::load(&yaml_path)?;

    // Remove first two photos from page 0 to create two unplaced photos.
    if state.layout.is_empty() || state.layout[0].photos.len() < 2 {
        return Ok(());
    }
    let id_a = state.layout[0].photos.remove(0);
    let id_b = state.layout[0].photos.remove(0);
    if state.layout[0].slots.len() >= 2 {
        state.layout[0].slots.remove(0);
        state.layout[0].slots.remove(0);
    }
    state.save(&yaml_path)?;

    // Place only id_a via ids filter.
    let config = PlaceConfig {
        filters: vec![],
        ids: vec![id_a.clone()],
        dst: PlaceDst::Page(0),
    };
    let result = place(&project_root, &config)?;

    assert_eq!(
        result.result.photos_placed, 1,
        "only 1 of 2 unplaced photos should be placed"
    );

    let state_after = ProjectState::load(&yaml_path)?;
    assert!(
        state_after.layout[0].photos.contains(&id_a),
        "id_a should be placed"
    );
    // id_b must still be unplaced (not in any layout page).
    let placed: std::collections::HashSet<_> = state_after
        .layout
        .iter()
        .flat_map(|p| p.photos.iter())
        .collect();
    assert!(!placed.contains(&id_b), "id_b should remain unplaced");

    Ok(())
}

#[test]
fn test_place_into_new_page_at() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let yaml_path = project_root.join("testplace.yaml");
    let state_before = ProjectState::load(&yaml_path)?;
    let pages_before = state_before.layout.len();

    // Remove a photo from page 0 to create an unplaced photo
    let mut state_modified = state_before.clone();
    assert!(
        !state_modified.layout[0].photos.is_empty(),
        "need at least one photo on page 0"
    );
    let removed_photo = state_modified.layout[0].photos.remove(0);
    if !state_modified.layout[0].slots.is_empty() {
        state_modified.layout[0].slots.remove(0);
    }
    state_modified.save(&yaml_path)?;

    // Place the unplaced photo into a new page at position 1
    let config = PlaceConfig {
        filters: vec![],
        ids: vec![removed_photo.clone()],
        dst: PlaceDst::NewPageAt(1),
    };
    let result = place(&project_root, &config)?;

    assert_eq!(result.result.photos_placed, 1);
    assert_eq!(result.result.pages_affected, vec![1]);
    assert_eq!(result.result.pages_inserted, vec![1]);

    // Verify state: one more page, photo is on page 1
    let state_after = ProjectState::load(&yaml_path)?;
    assert_eq!(state_after.layout.len(), pages_before + 1);
    assert!(state_after.layout[1].photos.contains(&removed_photo));

    // Verify page numbering invariant: layout[i].page == i
    for (i, page) in state_after.layout.iter().enumerate() {
        assert_eq!(page.page, i, "page numbering invariant broken at index {i}");
    }

    Ok(())
}

#[test]
fn test_place_into_new_page_at_invalid_position() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_root = create_test_project_with_layout(&temp_dir)?;

    let yaml_path = project_root.join("testplace.yaml");
    let state = ProjectState::load(&yaml_path)?;
    let pages_count = state.layout.len();

    // Remove a photo to create an unplaced one
    let mut state_modified = state;
    state_modified.layout[0].photos.remove(0);
    if !state_modified.layout[0].slots.is_empty() {
        state_modified.layout[0].slots.remove(0);
    }
    state_modified.save(&yaml_path)?;

    // Try position beyond layout.len() (invalid)
    let config = PlaceConfig {
        filters: vec![],
        ids: vec![],
        dst: PlaceDst::NewPageAt(pages_count + 1),
    };
    let result = place(&project_root, &config);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid new page position")
    );

    Ok(())
}
