use super::BuildOptions;
use super::plan::BuildPlan;
use crate::commands::CommandOutput;
use crate::commands::build::BuildResult;
use crate::state_manager::StateManager;
use anyhow::Result;
use std::path::Path;
use tracing::info;

/// Performs the first build: generates layout for all photos and creates preview PDF.
pub fn first_build(
    mgr: StateManager,
    project_root: &Path,
    opts: BuildOptions,
) -> Result<CommandOutput<BuildResult>> {
    info!("First build: creating layout for all photos...");
    let output = BuildPlan::Full.run(mgr, project_root, opts)?;
    info!(
        "First build complete: {} pages generated",
        output.result.pages_rebuilt.len()
    );
    Ok(output)
}
