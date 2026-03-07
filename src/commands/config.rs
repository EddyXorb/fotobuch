//! `fotobuch config` command - Show current configuration

use anyhow::Result;
use std::path::Path;

use crate::models::ProjectConfig;

/// Resolved configuration with all defaults filled in
pub type ResolvedConfig = ProjectConfig;

/// Show current configuration (YAML + defaults)
///
/// # Steps
/// 1. Load fotobuch.yaml
/// 2. Resolve all defaults (fields not present in YAML get lib defaults)
/// 3. Output as formatted YAML with "# default" comments for non-explicit values
///
/// The output is valid YAML that can be copy-pasted into fotobuch.yaml
/// to override specific defaults.
///
/// # Arguments
/// * `project_root` - Path to the project directory
///
/// # Returns
/// * `ResolvedConfig` with all configuration values resolved
pub fn config(project_root: &Path) -> Result<ResolvedConfig> {
    // TODO: Implement config command
    // - Load fotobuch.yaml
    // - Deserialize with serde (handles defaults via #[serde(default)])
    // - Return complete config
    //
    // CLI layer will format output with "# default" annotations
    // for fields that weren't in the YAML

    let _ = project_root; // Silence unused warning

    // For now, return a placeholder with defaults
    Ok(ProjectConfig {
        book: crate::models::BookConfig {
            title: "placeholder".to_string(),
            page_width_mm: 420.0,
            page_height_mm: 297.0,
            bleed_mm: 3.0,
            margin_mm: 10.0,
            gap_mm: 5.0,
            bleed_threshold_mm: 3.0,
        },
        ga: Default::default(),
        preview: Default::default(),
    })
}
