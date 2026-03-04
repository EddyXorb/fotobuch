//! Domain types for the photobook layout solver.
//!
//! This module contains the core data structures without any behavior:
//! - `Photo`: Photo metadata (aspect ratio, area weight, group, timestamp)
//! - `Canvas`: Canvas dimensions and spacing parameters
//! - `PhotoPlacement`: Position and size of a photo on the canvas
//! - `LayoutResult`: Complete layout with all photo placements
//! - `FitnessWeights`: Weights for the genetic algorithm fitness function

mod canvas;
mod layout;
mod photo;
mod weights;

pub use canvas::Canvas;
pub use layout::{LayoutResult, PhotoPlacement};
pub use photo::Photo;
pub use weights::FitnessWeights;
