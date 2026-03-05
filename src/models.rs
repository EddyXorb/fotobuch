//! Domain models for the photobook layout solver.
//!
//! This module contains all data structures used throughout the application:
//! - **Configuration**: Canvas, FitnessWeights, GaConfig, IslandConfig
//! - **Photos**: Photo, PhotoInfo, ScannedPhoto, PhotoGroup
//! - **Layout**: PhotoPlacement, PageLayout, BookLayout
//! - **Request**: SolverRequest

mod book_layout;
mod canvas;
mod ga_config;
mod layout;
mod photo;
mod photo_group;
mod request;
mod weights;

// Re-export all public types
pub use book_layout::BookLayout;
pub use canvas::Canvas;
pub use ga_config::{GaConfig, IslandConfig};
pub use layout::{PageLayout, PhotoPlacement};
pub use photo::{Photo, PhotoInfo};
pub use photo_group::{PhotoGroup, ScannedPhoto};
pub use request::SolverRequest;
pub use weights::FitnessWeights;
