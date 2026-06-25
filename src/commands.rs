//! Command orchestration for the fotobuch CLI.
//!
//! Each module in this crate implements one CLI command and contains:
//! - Configuration structs for input
//! - Result structs for output
//! - A main function that orchestrates the necessary operations
//!
//! Commands never depend on CLI parsers (clap) - they work with plain Rust types.

pub mod add;
pub mod build;
pub mod config;
pub mod history;
pub mod page;
pub mod place;
pub mod project;
pub mod remove;
pub mod status;
pub mod undo;
pub mod unplace;

use std::path::Path;

use crate::models::ProjectState;
use crate::state_manager::StateManager;

/// Wraps a command result with an optional post-command state snapshot.
///
/// `changed_state` is `Some(s)` when the command committed a state change,
/// and `None` when the state was not modified (read-only commands, no-op writes).
pub struct CommandOutput<T> {
    pub result: T,
    pub changed_state: Option<ProjectState>,
}

/// Open → execute → finish wrapper for write commands.
///
/// The closure receives `&mut StateManager`, performs its work, and returns
/// `(commit_message, result)`. An empty commit message signals "nothing to do"
/// and `finish` will skip the commit when the state is unchanged.
pub fn run_write_command<T, E, F>(project_root: &Path, f: F) -> Result<CommandOutput<T>, E>
where
    E: From<anyhow::Error>,
    F: FnOnce(&mut StateManager) -> Result<(String, T), E>,
{
    let mut mgr = StateManager::open(project_root)?;
    let (commit_msg, result) = f(&mut mgr)?;
    let changed_state = mgr.finish(&commit_msg)?;
    Ok(CommandOutput {
        result,
        changed_state,
    })
}

impl<T: std::fmt::Debug> std::fmt::Debug for CommandOutput<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandOutput")
            .field("result", &self.result)
            .field("changed_state", &self.changed_state)
            .finish()
    }
}

pub use add::{AddConfig, AddResult, GroupSummary, add};
pub use build::{BuildConfig, BuildPlan, BuildResult, DpiWarning, build};
pub use config::{ConfigResult, current_config, render_config};
pub use history::{HistoryConfig, HistoryEntry, history};
pub use place::{PlaceConfig, PlaceDst, PlaceResult, place};
pub use remove::{RemoveConfig, RemoveResult, RemoveTarget, remove};
pub use status::{PageDetail, ProjectStatus, StatusConfig, StatusResult, StatusSlotInfo, status};
pub use undo::{UndoConfig, UndoResult, redo, undo};
pub use unplace::{UnplaceResult, unplace};
