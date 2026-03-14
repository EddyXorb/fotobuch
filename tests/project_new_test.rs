//! Integration tests for `fotobuch project new` command

use anyhow::Result;
use photobook_solver::commands::project::new::{NewConfig, project_new, validate_project_name};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_project_new_mode1_creates_complete_structure() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let config = NewConfig {
        name: "vacation2024".to_string(),
        width_mm: 1234.0,
        height_mm: 1234.0,
        bleed_mm: 1234.0,
        quiet: true,
    };

    let result = project_new(temp_dir.path(), &config)?;

    // Check that directory was created
    assert!(result.project_root.exists());
    assert!(result.project_root.is_dir());

    // Check branch name
    assert_eq!(result.branch, "fotobuch/vacation2024");

    // Check YAML file exists and has correct content
    assert!(result.yaml_path.exists());
    let yaml_content = fs::read_to_string(&result.yaml_path)?;
    assert!(yaml_content.contains("page_width_mm: 1234"));
    assert!(yaml_content.contains("page_height_mm: 1234"));
    assert!(yaml_content.contains("bleed_mm: 1234"));
    assert!(yaml_content.contains("title: vacation2024"));

    // Check Typst template exists and has placeholders replaced
    assert!(result.typ_path.exists());
    let typ_content = fs::read_to_string(&result.typ_path)?;
    assert!(typ_content.contains(r#"project_name = "vacation2024""#));
    assert!(!typ_content.contains("{project_name}"));

    // Check .gitignore exists and has correct entries
    let gitignore_path = result.project_root.join(".gitignore");
    assert!(gitignore_path.exists());
    let gitignore_content = fs::read_to_string(gitignore_path)?;
    assert!(gitignore_content.contains(".fotobuch/"));
    assert!(gitignore_content.contains("*.pdf"));
    assert!(gitignore_content.contains("final.typ"));

    // Check cache directories exist
    let cache_preview = result
        .project_root
        .join(".fotobuch/cache/vacation2024/preview");
    let cache_final = result
        .project_root
        .join(".fotobuch/cache/vacation2024/final");
    assert!(cache_preview.exists());
    assert!(cache_preview.is_dir());
    assert!(cache_final.exists());
    assert!(cache_final.is_dir());

    // Check git repository was initialized
    assert!(result.project_root.join(".git").exists());

    // Verify we're on the correct branch using git2
    let repo = git2::Repository::open(&result.project_root)?;
    let head = repo.head()?;
    assert!(head.is_branch());
    assert_eq!(head.shorthand(), Some("fotobuch/vacation2024"));

    Ok(())
}

#[test]
fn test_project_new_mode2_creates_additional_project() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create first project
    let config1 = NewConfig {
        name: "first".to_string(),
        width_mm: 200.0,
        height_mm: 250.0,
        bleed_mm: 2.0,
        quiet: true,
    };
    let result1 = project_new(temp_dir.path(), &config1)?;

    // Create second project in same repository
    let config2 = NewConfig {
        name: "second".to_string(),
        width_mm: 180.0,
        height_mm: 240.0,
        bleed_mm: 4.0,
        quiet: true,
    };
    let result2 = project_new(&result1.project_root, &config2)?;

    // Both projects should share the same root
    assert_eq!(result1.project_root, result2.project_root);

    // Check that second project files exist
    assert!(result2.yaml_path.exists());
    assert!(result2.typ_path.exists());
    assert_eq!(result2.branch, "fotobuch/second");

    // Verify second project YAML has correct dimensions
    let yaml2_content = fs::read_to_string(&result2.yaml_path)?;
    assert!(yaml2_content.contains("page_width_mm: 180"));
    assert!(yaml2_content.contains("page_height_mm: 240"));
    assert!(yaml2_content.contains("bleed_mm: 4"));

    // Verify we're on second branch
    let repo = git2::Repository::open(&result2.project_root)?;
    let head = repo.head()?;
    assert_eq!(head.shorthand(), Some("fotobuch/second"));

    // Verify both branches exist
    let _ = repo.find_branch("fotobuch/first", git2::BranchType::Local)?;
    let _ = repo.find_branch("fotobuch/second", git2::BranchType::Local)?;

    Ok(())
}

#[test]
fn test_project_new_rejects_duplicate_name() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let config = NewConfig {
        name: "project".to_string(),
        width_mm: 210.0,
        height_mm: 297.0,
        bleed_mm: 3.0,
        quiet: true,
    };

    // Create first project
    project_new(temp_dir.path(), &config)?;

    // Try to create project with same name - should fail
    let result = project_new(temp_dir.path(), &config);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_validate_project_name_comprehensive() {
    // Valid names
    assert!(validate_project_name("simple").is_ok());
    assert!(validate_project_name("with-dashes").is_ok());
    assert!(validate_project_name("with_underscores").is_ok());
    assert!(validate_project_name("with.dots").is_ok());
    assert!(validate_project_name("Mixed123").is_ok());

    // Invalid: empty
    assert!(validate_project_name("").is_err());

    // Invalid: starts with non-letter
    assert!(validate_project_name("1project").is_err());
    assert!(validate_project_name("-project").is_err());
    assert!(validate_project_name("_project").is_err());

    // Invalid: reserved name
    assert!(validate_project_name("fotobuch").is_err());

    // Invalid: path traversal
    assert!(validate_project_name("..").is_err());
    assert!(validate_project_name("foo..bar").is_err());

    // Invalid: special characters
    assert!(validate_project_name("foo bar").is_err());
    assert!(validate_project_name("foo/bar").is_err());
    assert!(validate_project_name("foo\\bar").is_err());
    assert!(validate_project_name("foo@bar").is_err());

    // Invalid: too long
    let long_name = "a".repeat(51);
    assert!(validate_project_name(&long_name).is_err());
}

#[test]
fn test_project_new_with_different_page_dimensions() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let test_cases = vec![
        ("a4", 210.0, 297.0, 3.0),
        ("square", 200.0, 200.0, 5.0),
        ("landscape", 297.0, 210.0, 2.0),
    ];

    for (name, width, height, bleed) in test_cases {
        let config = NewConfig {
            name: name.to_string(),
            width_mm: width,
            height_mm: height,
            bleed_mm: bleed,
            quiet: true,
        };

        let result = project_new(temp_dir.path(), &config)?;

        let yaml_content = fs::read_to_string(&result.yaml_path)?;
        assert!(yaml_content.contains(&format!("page_width_mm: {}", width)));
        assert!(yaml_content.contains(&format!("page_height_mm: {}", height)));
        assert!(yaml_content.contains(&format!("bleed_mm: {}", bleed)));
    }

    Ok(())
}
