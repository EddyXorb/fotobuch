//! Handler for `fotobuch config` subcommands.

use anyhow::{Context, Result};
use fotobuch::commands;
use tracing::info;

fn project_root() -> Result<std::path::PathBuf> {
    std::env::current_dir().context("Failed to determine current directory")
}

/// Handler for `fotobuch config show`.
pub fn handle_show() -> Result<()> {
    let result = commands::config(&project_root()?)?;
    let output = commands::render_config(&result)?;
    info!("{}", output);
    Ok(())
}

/// Handler for `fotobuch config set <key> <value>`.
pub fn handle_set(key: &str, value: &str) -> Result<()> {
    let result = commands::config::config_set(&project_root()?, key, value)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "{}: {} → {}",
        result.key, result.old_value, result.new_value
    );
    Ok(())
}
