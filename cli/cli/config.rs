//! Handler for `fotobuch config` command

use anyhow::Context;
use anyhow::Result;
use fotobuch::commands;
use tracing::info;

pub fn handle() -> Result<()> {
    let project_root = std::env::current_dir().context("Failed to determine current directory")?;

    let result = commands::config(&project_root)?;
    let output = commands::render_config(&result)?;
    info!("{}", output);

    Ok(())
}
