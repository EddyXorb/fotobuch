//! `fotobuch project new` command - Create a new photobook project
//!
//! This command supports two modes:
//! - Mode 1: First project (no fotobuch/* branches exist) - creates new directory
//! - Mode 2: Additional project (fotobuch/* branches exist) - creates in repository root

mod template;
mod validation;
mod yaml;

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

pub use validation::validate_project_name;

use crate::git;

/// Configuration for creating a new project
#[derive(Debug, Clone)]
pub struct NewConfig {
    /// Project name (becomes branch name fotobuch/<name>)
    pub name: String,
    /// Page width in millimeters
    pub width_mm: f64,
    /// Page height in millimeters
    pub height_mm: f64,
    /// Bleed distance in millimeters
    pub bleed_mm: f64,
    /// Quiet mode (suppress welcome message)
    pub quiet: bool,
    /// Create project with active cover
    pub with_cover: bool,
    /// Cover width (defaults to width_mm * 2 if with_cover and not provided)
    pub cover_width_mm: Option<f64>,
    /// Cover height (defaults to height_mm if with_cover and not provided)
    pub cover_height_mm: Option<f64>,
    /// Spine auto mode: mm per 10 inner pages
    pub spine_grow_per_10_pages_mm: Option<f64>,
    /// Spine fixed mode: fixed width in mm
    pub spine_mm: Option<f64>,
    /// Inner margin in millimeters
    pub margin_mm: f64,
}

/// Result of project creation
#[derive(Debug)]
pub struct NewResult {
    /// Path to the project root (directory or repository)
    pub project_root: PathBuf,
    /// Branch name (fotobuch/<name>)
    pub branch: String,
    /// Path to the YAML file
    pub yaml_path: PathBuf,
    /// Path to the Typst template file
    pub typ_path: PathBuf,
}

const WELCOME_MESSAGE: &str = r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                    Welcome to fotobuch!                                      ║
╚══════════════════════════════════════════════════════════════════════════════╝

Your new photobook project has been created! Here's what you need to know:

📁 Project Structure:
   - <name>.yaml: Contains your project configuration and layout
   - <name>.typ:  Typst template for rendering your photobook
   - .fotobuch/:  Cache directory (not tracked in git)

📝 Workflow:
   1. fotobuch add <photos>     - Add photos to your project
   2. fotobuch build           - Generate preview PDF
   3. fotobuch place <photo>   - Manually adjust photo placement
   4. fotobuch build --release - Generate final PDF for printing

🔧 Configuration:
   You can edit <name>.yaml and <name>.typ to customize your photobook.
   All changes in between two fotobuch-command calls are tracked in git, so you can undo anything!

💡 Tips:
   - The project directory can be renamed, but don't rename .yaml or .typ files
   - Use 'git log' to see your project history
   - Each project lives on its own branch: fotobuch/<name>

Happy photobook making! 📷✨
"#;

/// Create a new photobook project
///
/// Automatically detects mode:
/// - Mode 1 (first project): Creates new directory under `parent_dir_or_root`
/// - Mode 2 (additional project): Creates in repository root at `parent_dir_or_root`
pub fn project_new(parent_dir_or_root: &Path, config: &NewConfig) -> Result<NewResult> {
    validate_project_name(&config.name)?;

    // Detect mode: check if we're already in a repo with fotobuch branches
    let mode = if git::is_git_repo(parent_dir_or_root) {
        let repo = git::open_repo(parent_dir_or_root)?;
        let branches = git::list_branches_with_prefix(&repo, "fotobuch/")?;

        if branches.is_empty() {
            Mode::FirstProject
        } else {
            Mode::AdditionalProject
        }
    } else {
        Mode::FirstProject
    };

    match mode {
        Mode::FirstProject => create_first_project(parent_dir_or_root, config),
        Mode::AdditionalProject => create_additional_project(parent_dir_or_root, config),
    }
}

#[derive(Debug, Clone, Copy)]
enum Mode {
    FirstProject,
    AdditionalProject,
}

/// Mode 1: Create first project in new directory
fn create_first_project(parent_dir: &Path, config: &NewConfig) -> Result<NewResult> {
    let project_root = parent_dir.join(&config.name);

    // 1. Create directory
    if project_root.exists() {
        bail!("Directory '{}' already exists", config.name);
    }
    fs::create_dir(&project_root)
        .with_context(|| format!("Failed to create directory {:?}", project_root))?;

    // 2. Initialize git
    let repo = git::init_repo(&project_root)?;

    // 3. Write .gitignore
    let gitignore_path = project_root.join(".gitignore");
    let gitignore_content = r#"
.fotobuch/
*.pdf
final.typ
log*
*.yaml
*.typ
"#;
    fs::write(&gitignore_path, gitignore_content)
        .with_context(|| format!("Failed to write .gitignore to {:?}", gitignore_path))?;

    // 4. Write YAML
    let yaml_path = project_root.join(format!("{}.yaml", config.name));
    let state = yaml::generate_default_state(config);
    yaml::write_yaml(&yaml_path, &state)?;

    // 5. Write Typst template
    let typ_path = project_root.join(format!("{}.typ", config.name));
    let template = template::generate_template(&config.name)?;
    fs::write(&typ_path, template)
        .with_context(|| format!("Failed to write template to {:?}", typ_path))?;

    // 6. Create cache directories
    let cache_base = project_root.join(".fotobuch/cache").join(&config.name);
    fs::create_dir_all(cache_base.join("preview"))
        .context("Failed to create preview cache directory")?;
    fs::create_dir_all(cache_base.join("final"))
        .context("Failed to create final cache directory")?;

    // 7. Create branch and initial commit
    let branch_name = format!("fotobuch/{}", config.name);
    let yaml_name = format!("{}.yaml", config.name);
    let typ_name = format!("{}.typ", config.name);

    git::stage_and_commit(
        &repo,
        &[".gitignore", &yaml_name, &typ_name],
        &format!(
            "new: {}, {}x{}mm, {}mm bleed",
            config.name, config.width_mm, config.height_mm, config.bleed_mm
        ),
    )?;

    // 8. Create and switch to project branch
    git::create_branch(&repo, &branch_name)?;

    // 9. Print welcome message
    if !config.quiet {
        println!("{}", WELCOME_MESSAGE); //intentionally avoid logging this
    }

    Ok(NewResult {
        project_root,
        branch: branch_name,
        yaml_path,
        typ_path,
    })
}

/// Mode 2: Create additional project in existing repository
fn create_additional_project(repo_root: &Path, config: &NewConfig) -> Result<NewResult> {
    let repo = git::open_repo(repo_root)?;

    // 1. Check if branch already exists
    let branch_name = format!("fotobuch/{}", config.name);
    let branches = git::list_branches_with_prefix(&repo, "fotobuch/")?;
    if branches.contains(&branch_name) {
        bail!("Project '{}' already exists", config.name);
    }

    // 2. Get current branch to know which files to unstage
    let current_branch = git::current_branch(&repo).ok();
    let old_project_name = current_branch
        .as_ref()
        .and_then(|b| b.strip_prefix("fotobuch/"))
        .map(|s| s.to_owned());

    // 3. Write YAML
    let yaml_path = repo_root.join(format!("{}.yaml", config.name));
    let state = yaml::generate_default_state(config);
    yaml::write_yaml(&yaml_path, &state)?;

    // 4. Write Typst template
    let typ_path = repo_root.join(format!("{}.typ", config.name));
    let template = template::generate_template(&config.name)?;
    fs::write(&typ_path, template)
        .with_context(|| format!("Failed to write template to {:?}", typ_path))?;

    // 5. Create cache directories
    let cache_base = repo_root.join(".fotobuch/cache").join(&config.name);
    fs::create_dir_all(cache_base.join("preview"))
        .context("Failed to create preview cache directory")?;
    fs::create_dir_all(cache_base.join("final"))
        .context("Failed to create final cache directory")?;

    // 6. Create new branch
    git::create_branch(&repo, &branch_name)?;

    // 7. Remove old project files from index (if any)
    if let Some(old_name) = old_project_name {
        let old_yaml = format!("{}.yaml", old_name);
        let old_typ = format!("{}.typ", old_name);

        // Try to remove from index, but don't fail if they don't exist
        let mut index = repo.index().context("Failed to get repository index")?;
        let _ = index.remove_path(Path::new(&old_yaml));
        let _ = index.remove_path(Path::new(&old_typ));
        index.write().context("Failed to write index")?;
    }

    // 8. Stage new project files and commit
    let yaml_name = format!("{}.yaml", config.name);
    let typ_name = format!("{}.typ", config.name);

    git::stage_and_commit(
        &repo,
        &[&yaml_name, &typ_name],
        &format!(
            "new: {}, {}x{}mm, {}mm bleed",
            config.name, config.width_mm, config.height_mm, config.bleed_mm
        ),
    )?;

    Ok(NewResult {
        project_root: repo_root.to_path_buf(),
        branch: branch_name,
        yaml_path,
        typ_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_first_project_creates_directory_structure() {
        let temp_dir = TempDir::new().unwrap();
        let config = NewConfig {
            name: "vacation".to_string(),
            width_mm: 210.0,
            height_mm: 297.0,
            bleed_mm: 3.0,
            quiet: true,
            with_cover: false,
            cover_width_mm: None,
            cover_height_mm: None,
            spine_grow_per_10_pages_mm: None,
            spine_mm: None,
            margin_mm: 0.0,
        };

        let result = project_new(temp_dir.path(), &config).unwrap();

        assert!(result.project_root.exists());
        assert!(result.yaml_path.exists());
        assert!(result.typ_path.exists());
        assert_eq!(result.branch, "fotobuch/vacation");

        // Check cache directories
        let cache_base = result.project_root.join(".fotobuch/cache/vacation");
        assert!(cache_base.join("preview").exists());
        assert!(cache_base.join("final").exists());

        // Check .gitignore
        let gitignore = result.project_root.join(".gitignore");
        assert!(gitignore.exists());
        let content = fs::read_to_string(gitignore).unwrap();
        assert!(content.contains(".fotobuch/"));
        assert!(content.contains("*.pdf"));
        assert!(content.contains("final.typ"));
        assert!(content.contains("log*"));
    }

    #[test]
    fn test_yaml_contains_correct_dimensions() {
        let temp_dir = TempDir::new().unwrap();
        let config = NewConfig {
            name: "test".to_string(),
            width_mm: 200.0,
            height_mm: 250.0,
            bleed_mm: 5.0,
            quiet: true,
            with_cover: false,
            cover_width_mm: None,
            cover_height_mm: None,
            spine_grow_per_10_pages_mm: None,
            spine_mm: None,
            margin_mm: 0.0,
        };

        let result = project_new(temp_dir.path(), &config).unwrap();

        let yaml_content = fs::read_to_string(&result.yaml_path).unwrap();
        assert!(yaml_content.contains("page_width_mm: 200"));
        assert!(yaml_content.contains("page_height_mm: 250"));
        assert!(yaml_content.contains("bleed_mm: 5"));
    }

    #[test]
    fn test_template_has_placeholders_replaced() {
        let temp_dir = TempDir::new().unwrap();
        let config = NewConfig {
            name: "mybook".to_string(),
            width_mm: 210.0,
            height_mm: 297.0,
            bleed_mm: 3.0,
            quiet: true,
            with_cover: false,
            cover_width_mm: None,
            cover_height_mm: None,
            spine_grow_per_10_pages_mm: None,
            spine_mm: None,
            margin_mm: 0.0,
        };

        let result = project_new(temp_dir.path(), &config).unwrap();

        let typ_content = fs::read_to_string(&result.typ_path).unwrap();
        assert!(typ_content.contains(r#"project_name = "mybook""#));
        assert!(!typ_content.contains("{project_name}"));
    }

    #[test]
    fn test_invalid_name_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let config = NewConfig {
            name: "1invalid".to_string(), // starts with number
            width_mm: 210.0,
            height_mm: 297.0,
            bleed_mm: 3.0,
            quiet: true,
            with_cover: false,
            cover_width_mm: None,
            cover_height_mm: None,
            spine_grow_per_10_pages_mm: None,
            spine_mm: None,
            margin_mm: 0.0,
        };

        let result = project_new(temp_dir.path(), &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_directory_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let config = NewConfig {
            name: "test".to_string(),
            width_mm: 210.0,
            height_mm: 297.0,
            bleed_mm: 3.0,
            quiet: true,
            with_cover: false,
            cover_width_mm: None,
            cover_height_mm: None,
            spine_grow_per_10_pages_mm: None,
            spine_mm: None,
            margin_mm: 0.0,
        };

        // Create first project
        project_new(temp_dir.path(), &config).unwrap();

        // Try to create same project again
        let result = project_new(temp_dir.path(), &config);
        assert!(result.is_err());
    }
}
