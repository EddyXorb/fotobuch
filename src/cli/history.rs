//! Handler for `fotobuch history` command

use anyhow::Context;
use anyhow::Result;
use photobook_solver::commands;
use tracing::info;

pub fn handle(count: usize) -> Result<()> {
    let project_root = std::env::current_dir().context("Failed to determine current directory")?;

    let entries = commands::history(&project_root, count)?;

    if entries.is_empty() {
        info!("ℹ️  No history available (not a git repository or no commits yet).");
    } else {
        for entry in entries {
            let formatted_ts = entry.timestamp.format("%Y-%m-%d %H:%M");
            info!("{}  {}", formatted_ts, entry.message);
        }
    }

    Ok(())
}
