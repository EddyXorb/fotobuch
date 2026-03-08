use super::multipage_build::{multipage_build, MultiPageParams};
use super::BuildResult;
use crate::state_manager::StateManager;
use anyhow::Result;
use std::path::Path;

/// Performs the first build: generates layout for all photos and creates preview PDF.
pub fn first_build(mgr: StateManager, project_root: &Path) -> Result<BuildResult> {
    println!("First build: creating layout for all photos...");

    let groups = mgr.state.photos.clone();
    
    let result = multipage_build(
        mgr,
        project_root,
        MultiPageParams {
            groups: &groups,
            range: None,
            flex: 0,
            custom_config: None,
            commit_message: "build: initial layout".to_string(),
            images_processed: 0,
        },
    )?;

    println!("First build complete: {} pages generated", result.pages_rebuilt.len());
    
    Ok(result)
}
