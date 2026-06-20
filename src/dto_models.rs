//! Domain models for the photobook layout solver.
//!
//! This module contains all data structures used throughout the application:
//! - **Configuration**: Canvas, FitnessWeights, GaConfig, IslandConfig
//! - **Photos**: Photo, PhotoInfo, ScannedPhoto, PhotoGroup
//! - **Layout**: PhotoPlacement, SolverPageLayout, BookLayout
//! - **Request**: SolverRequest
mod config;
mod cover;
mod layout;
mod photos;
mod state;

pub use config::{
    AppendixConfig, BookConfig, BookLayoutSolverConfig, CanvasConfig, CoverConfig, CoverMode,
    FitnessWeights, GaConfig, PreviewConfig, ProjectConfig, SpineConfig, ValidationError,
};
pub use cover::CoverGeometry;
pub use layout::{LayoutPage, PageMode, Slot};
pub use photos::{PhotoFile, PhotoGroup, build_photo_index};
pub use state::ProjectState;
