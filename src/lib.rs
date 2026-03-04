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

// Re-export commonly used types
pub use model::{Canvas, FitnessWeights, LayoutResult, Photo, PhotoPlacement};
pub use solver::{run_ga, solve_layout, total_cost, Cut, Node, SlicingTree};

