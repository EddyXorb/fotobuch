//! `fotobuch new` command - Create a new photobook project

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Configuration for creating a new project
#[derive(Debug, Clone)]
pub struct NewConfig {
    /// Project name (becomes directory name)
    pub name: String,
    /// Page width in millimeters
    pub width_mm: f64,
    /// Page height in millimeters
    pub height_mm: f64,
    /// Bleed distance in millimeters
    pub bleed_mm: f64,
}

/// Result of project creation
#[derive(Debug)]
pub struct NewResult {
    /// Path to the created project directory
    pub project_path: PathBuf,
    /// Book dimensions summary (for display)
    pub dimensions: String,
}

/// Create a new photobook project
///
/// # Steps
/// 1. Creates directory `<parent_dir>/<name>/`
/// 2. Creates `fotobuch.yaml` with page dimensions (no photos/pages yet)
/// 3. Creates `.fotobuch/cache/` with preview/ and final/ subdirectories
/// 4. `git init --initial-branch=fotobuch` + `.gitignore`
/// 5. Initial commit with `fotobuch.yaml`
///
/// # Arguments
/// * `parent_dir` - Parent directory where the project folder will be created
/// * `config` - Project configuration
///
/// # Returns
/// * `NewResult` with project path and dimensions, or error if creation fails
pub fn new(parent_dir: &Path, config: &NewConfig) -> Result<NewResult> {
    let project_path = parent_dir.join(&config.name);

    // TODO: Implement project creation
    // - Create directory structure
    // - Generate fotobuch.yaml with config
    // - Create cache directories
    // - Initialize git repository
    // - Create initial commit

    let dimensions = format!(
        "{}x{}mm, {}mm bleed",
        config.width_mm, config.height_mm, config.bleed_mm
    );

    Ok(NewResult {
        project_path,
        dimensions,
    })
}
