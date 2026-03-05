//! Domain models for the photobook layout solver.
//!
//! This module contains all data structures used throughout the application:
//! - **Configuration**: Canvas, FitnessWeights, GaConfig, IslandConfig
//! - **Photos**: Photo, PhotoInfo, ScannedPhoto, PhotoGroup
//! - **Layout**: PhotoPlacement, LayoutResult
//! - **Request**: SolverRequest

mod canvas;
mod ga_config;
mod layout;
mod photo;
mod photo_group;
mod request;
mod weights;

// Re-export all public types
pub use canvas::Canvas;
pub use ga_config::{GaConfig, IslandConfig};
pub use layout::{LayoutResult, PhotoPlacement};
pub use photo::{Photo, PhotoInfo};
pub use photo_group::{PhotoGroup, ScannedPhoto};
pub use request::SolverRequest;
pub use weights::FitnessWeights;
