//! Handlers for `fotobuch undo` and `fotobuch redo`.

use anyhow::{Context, Result};
use fotobuch::commands;
use tracing::info;

pub fn handle_undo(steps: usize) -> Result<()> {
    let project_root = std::env::current_dir().context("Failed to determine current directory")?;
    let result = commands::undo(&project_root, steps)?;

    if result.wip_committed {
        info!("  Auto-committed uncommitted changes as \"wip: before undo\".");
    }
    info!("  Undone:  {}", result.undone_message);
    info!("  Now at:  {}", result.current_message);
    Ok(())
}

pub fn handle_redo(steps: usize) -> Result<()> {
    let project_root = std::env::current_dir().context("Failed to determine current directory")?;
    let result = commands::redo(&project_root, steps)?;

    info!("  Redone:  {}", result.undone_message);
    info!("  Now at:  {}", result.current_message);
    Ok(())
}
