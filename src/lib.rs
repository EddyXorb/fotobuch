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
//!     population: 300,
//!     generations: 100,
//!     mutation_rate: 0.2,
//!     crossover_rate: 0.7,
//!     tournament_size: 3,
//!     elitism_ratio: 0.05,
//!     weights: FitnessWeights::default(),
//!     timeout: Some(std::time::Duration::from_secs(30)),
//!     island_config: Some(IslandConfig::default()),
//! };
//!
//! let request = SolverRequest::new(
//!     PathBuf::from("photos/"),
//!     PathBuf::from("output.pdf"),
//!     Canvas::new(297.0, 210.0, 2.0, 5.0),
//!     ga_config,
//!     42,
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

pub mod input;
pub mod models;
pub mod output;
pub mod solver;

// Re-export core API types for convenience
pub use input::loader::load_photos_from_dir;
pub use models::{BookLayout, Canvas, FitnessWeights, GaConfig, IslandConfig, PageLayout, Photo, SolverRequest};
pub use output::{export_json, export_pdf, export_typst};
pub use solver::run_solver;
