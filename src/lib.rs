//! Photobook Layout Solver
//!
//! A genetic algorithm-based layout solver for photo books using slicing tree structures.
//! 
//! # Architecture
//!
//! - `models`: Domain types (Photo, Canvas, Layout, PhotoGroup, SolverRequest, etc.)
//! - `solver`: Core algorithm (slicing trees, layout solver, genetic algorithm)
//! - `input`: Data input and scanning
//! - `output`: Result export (JSON, Typst, PDF)
//! - `scanner`: Photo directory scanner

pub mod models;
pub mod solver;
pub mod input;
pub mod output;

// Re-export commonly used types
pub use models::{Canvas, FitnessWeights, LayoutResult, Photo, PhotoPlacement, PhotoGroup, PhotoInfo, SolverRequest};
pub use solver::{run_ga, run_solver, solve_layout, total_cost, GaConfig, IslandConfig};
pub use input::load_photos_from_dir;
pub use output::{export_json, export_typst, export_pdf};

