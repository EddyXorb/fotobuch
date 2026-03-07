//! Project state management and YAML operations.
//!
//! This module contains structures and functions for managing the fotobuch project state,
//! including loading/saving fotobuch.yaml, photo groups, and layout information.

pub mod state;
// pub mod git;      // Will be added in Commit 6
// pub mod timestamp; // Will be added in Commit 4

pub use state::{LayoutPage, PhotoFile, PhotoGroup, ProjectState, Slot};
