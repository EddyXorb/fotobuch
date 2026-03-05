//! Photobook Layout Solver
//!
//! A genetic algorithm-based layout solver for photo books using slicing tree structures.
//! 
//! # Architecture
//!
//! - `model/`: Domain types (Photo, Canvas, Layout, etc.)
//! - `solver/`: Core algorithm (slicing trees, layout solver, genetic algorithm)
//! - `input/`: Data input (CLI, EXIF, manifest)
//! - `output/`: Result export (JSON, Typst)

pub mod model;
pub mod solver;
pub mod input;
pub mod output;

// Legacy modules (to be migrated or removed)
mod scanner;
mod models; // Old models module used by scanner

// Re-export commonly used types
pub use model::{Canvas, FitnessWeights, LayoutResult, Photo, PhotoPlacement};
pub use solver::{run_ga, run_island_ga, solve_layout, total_cost, GaConfig, IslandConfig};
pub use input::{load_photos_from_dir, PhotoInfo};
pub use output::{export_json, export_typst};

