use anyhow::{Context, Result};
use std::path::Path;

use super::ProjectState;

/// Read project state from a YAML file path.
pub fn read_state_yaml(path: &Path) -> Result<ProjectState> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let state: ProjectState = serde_yaml::from_str(&contents)
        .with_context(|| format!("Failed to parse YAML from {}", path.display()))?;

    Ok(state)
}

/// Write project state to a YAML file path.
pub fn write_state_yaml(state: &ProjectState, path: &Path) -> Result<()> {
    let yaml = serde_yaml::to_string(state).context("Failed to serialize project state to YAML")?;

    std::fs::write(path, yaml).with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}
