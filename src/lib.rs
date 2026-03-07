//! Photobook Layout Solver
//!
//! A genetic algorithm-based layout solver for photo books using slicing tree structures.
//!
//! # Quick Start
//!
//! ```no_run
//! use photobook_solver::{Canvas, FitnessWeights, GaConfig, IslandConfig, SolverRequest, run_solver};
//! use std::path::PathBuf;
//!
//! let ga_config = GaConfig {
//!     seed: 42,
//!     ..GaConfig::default()
//! };
//!
//! let request = SolverRequest::new(
//!     PathBuf::from("photos/"),
//!     PathBuf::from("output.pdf"),
//!     Canvas::new(297.0, 210.0, 2.0, 5.0),
//!     ga_config,
//! );
//!
//! run_solver(&request).expect("Solver failed");
//! ```
//!
//! # Architecture
//!
//! - `models`: Domain types (Photo, Canvas, Layout, PhotoGroup, SolverRequest, etc.)
//! - `solver`: Core algorithm (slicing trees, layout solver, genetic algorithm)
//! - `input`: Data input and scanning
//! - `output`: Result export (JSON, Typst, PDF)
//! - `commands`: Command orchestration (CLI command implementations)

pub mod commands;
pub mod input;
pub mod dto_models;
pub mod output;
pub mod project;
pub mod solver;

#[cfg(test)]
mod test_fixtures;

// Re-export core API types for convenience
pub use input::loader::load_photos_from_dir;

pub use output::{export_json, export_pdf, export_typst};
pub use solver::run_solver;
