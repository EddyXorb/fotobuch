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
pub mod place;
pub mod project;
pub mod rebuild;
pub mod remove;
pub mod status;

pub use add::{AddConfig, AddResult, GroupSummary, add};
pub use build::{BuildConfig, BuildResult, DpiWarning, build};
pub use config::{ResolvedConfig, config};
pub use history::{HistoryEntry, history};
pub use place::{PlaceConfig, PlaceResult, place};
pub use project::new::{project_new, validate_project_name};
pub use rebuild::{RebuildScope, rebuild};
pub use remove::{RemoveConfig, RemoveResult, remove};
pub use status::{PageStatus, SlotInfo, StatusReport, status};
