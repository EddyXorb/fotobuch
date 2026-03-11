//! Handler for `fotobuch history` command

use anyhow::Context;
use anyhow::Result;
use photobook_solver::commands;
use tracing::info;

pub fn handle() -> Result<()> {
    let project_root = std::env::current_dir()
        .context("Failed to determine current directory")?;

    let entries = commands::history(&project_root)?;

    if entries.is_empty() {
        info!("ℹ️  No history available (not a git repository or no commits yet).");
    } else {
        for entry in entries {
            // Format timestamp: "2024-03-07 14:22 +0100" -> "2024-03-07 14:22"
            let ts_parts: Vec<&str> = entry.timestamp.split_whitespace().collect();
            let formatted_ts = if ts_parts.len() >= 2 {
                format!("{} {}", ts_parts[0], ts_parts[1])
            } else {
                entry.timestamp.clone()
            };
            info!("{}  {}", formatted_ts, entry.message);
        }
    }

    Ok(())
}
