//! Domain models for the photobook layout solver.
//!
//! This module contains all data structures used throughout the application:
//! - **Solver models**: Photo, Canvas, PhotoPlacement, LayoutResult, FitnessWeights
//! - **Scanner models**: ScannedPhoto, PhotoGroup
//! - **Bridge models**: PhotoInfo

mod canvas;
mod layout;
mod photo;
mod photo_group;
mod photo_info;
mod weights;

// Re-export all public types
pub use canvas::Canvas;
pub use layout::{LayoutResult, PhotoPlacement};
pub use photo::Photo;
pub use photo_group::{PhotoGroup, ScannedPhoto};
pub use photo_info::PhotoInfo;
pub use weights::FitnessWeights;
