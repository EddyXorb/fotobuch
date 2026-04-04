use super::BuildResult;
use super::core::multipage_build::{MultiPageParams, multipage_build};
use crate::commands::CommandOutput;
use crate::state_manager::StateManager;
use anyhow::Result;
use std::path::Path;
use tracing::info;

/// Performs the first build: generates layout for all photos and creates preview PDF.
pub fn first_build(mgr: StateManager, project_root: &Path) -> Result<CommandOutput<BuildResult>> {
    info!("First build: creating layout for all photos...");

    let groups = mgr.state.photos.clone();

    let output = multipage_build(
        mgr,
        project_root,
        MultiPageParams {
            groups: &groups,
            range: None,
            flex: 0,
            custom_config: None,
            commit_message: "build: initial layout".to_string(),
            images_processed: 0,
            always_commit: false,
        },
    )?;

    info!(
        "First build complete: {} pages generated",
        output.result.pages_rebuilt.len()
    );

    Ok(output)
}
