//! Project state management and YAML operations.
//!
//! This module contains structures and functions for managing the fotobuch project state,
//! including loading/saving fotobuch.yaml, photo groups, and layout information.

pub mod git;
pub mod state;
// pub mod timestamp; // Timestamp parsing already in scanner module

pub use state::{LayoutPage, PhotoFile, PhotoGroup, ProjectState, Slot};
