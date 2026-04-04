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
pub mod rebuild;
pub mod remove;
pub mod status;
pub mod undo;
pub mod unplace;

use crate::dto_models::ProjectState;

/// Wraps a command result together with the project state after the command.
/// Allows the GUI to update its state view after each state-modifying command.
pub struct CommandOutput<T> {
    pub result: T,
    pub state: ProjectState,
}

impl<T: std::fmt::Debug> std::fmt::Debug for CommandOutput<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandOutput")
            .field("result", &self.result)
            .field("state", &self.state)
            .finish()
    }
}

pub use add::{AddConfig, AddResult, GroupSummary, add};
pub use build::{BuildConfig, BuildResult, DpiWarning, build};
pub use config::{ConfigResult, config, render_config};
pub use history::{HistoryEntry, history};
pub use place::{PlaceConfig, PlaceResult, place};
pub use project::new::{project_new, validate_project_name};
pub use rebuild::{RebuildScope, rebuild};
pub use remove::{RemoveConfig, RemoveResult, remove};
pub use status::{PageDetail, ProjectState_, SlotInfo, StatusConfig, StatusReport, status};
pub use undo::{UndoResult, redo, undo};
