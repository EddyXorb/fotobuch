//! Command orchestration for the fotobuch CLI.
//!
//! Each module in this crate implements one CLI command and contains:
//! - Configuration structs for input
//! - Result structs for output
//! - A main function that orchestrates the necessary operations
//!
//! Commands never depend on CLI parsers (clap) - they work with plain Rust types.

#[path = "commands/add.rs"]
pub mod add;
#[path = "commands/build.rs"]
pub mod build;
#[path = "commands/new.rs"]
pub mod new;
#[path = "commands/place.rs"]
pub mod place;
#[path = "commands/remove.rs"]
pub mod remove;
#[path = "commands/status.rs"]
pub mod status;

pub use add::{add, AddConfig, AddResult, GroupSummary};
pub use build::{build, BuildConfig, BuildResult, DpiWarning};
pub use new::{new, NewConfig, NewResult};
pub use place::{place, PlaceConfig, PlaceResult};
pub use remove::{remove, RemoveConfig, RemoveResult};
pub use status::{status, PageStatus, SlotInfo, StatusReport};
