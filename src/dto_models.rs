//! Domain models for the photobook layout solver.
//!
//! This module contains all data structures used throughout the application:
//! - **Configuration**: Canvas, FitnessWeights, GaConfig, IslandConfig
//! - **Photos**: Photo, PhotoInfo, ScannedPhoto, PhotoGroup
//! - **Layout**: PhotoPlacement, PageLayout, BookLayout
//! - **Request**: SolverRequest
mod config;
mod layout;
mod photos;
mod state;

pub use config::{BookConfig, FitnessWeights, GaConfig, PreviewConfig, ProjectConfig};
pub use layout::{LayoutPage, Slot};
pub use photos::{PhotoFile, PhotoGroup};
pub use state::ProjectState;
