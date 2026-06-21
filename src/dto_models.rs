//! Domain models for the photobook layout solver.
//!
//! This module contains all data structures used throughout the application:
//! - **Configuration**: Canvas, FitnessWeights, PageLayoutSolverConfig, IslandConfig
//! - **Photos**: Photo, PhotoInfo, ScannedPhoto, PhotoGroup
//! - **Layout**: PhotoPlacement, SolverPageLayout, BookLayout
//! - **Request**: SolverRequest
mod config;
mod cover;
mod layout;
mod photos;
mod state;

/// Returns `width / height`. Single source of truth for all aspect-ratio calculations.
pub fn aspect_ratio(width: f64, height: f64) -> f64 {
    width / height
}

pub use config::{
    AppendixConfig, BookConfig, BookLayoutSolverConfig, CanvasConfig, CoverConfig, CoverMode,
    FitnessWeights, PageLayoutSolverConfig, PreviewConfig, ProjectConfig, SpineConfig,
    ValidationError, validate_book_layout_solver_config,
};
pub use cover::CoverGeometry;
pub use layout::{LayoutPage, PageMode, Slot};
pub use photos::{PhotoFile, PhotoGroup, build_photo_index};
pub use state::ProjectState;
