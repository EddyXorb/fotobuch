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
pub mod new;
pub mod place;
pub mod project;
pub mod rebuild;
pub mod remove;
pub mod status;

pub use add::{AddConfig, AddResult, GroupSummary};
pub use build::{build, BuildConfig, BuildResult, DpiWarning};
pub use config::{config, ResolvedConfig};
pub use history::{history, HistoryEntry};
pub use new::{new, NewConfig, NewResult};
pub use place::{place, PlaceConfig, PlaceResult};
pub use project::new::{project_new, validate_project_name};
pub use rebuild::{rebuild, RebuildScope};
pub use remove::{remove, RemoveConfig, RemoveResult};
pub use status::{status, PageStatus, SlotInfo, StatusReport};
