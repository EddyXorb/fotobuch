//! Configuration structures for the YAML project state.
//!
//! This module contains the configuration structures that are persisted in `fotobuch.yaml`
//! and also used internally throughout the application to minimize translation overhead.

mod appendix_config;
mod book_config;
mod book_layout_solver_config;
mod cover_config;
mod fitness_weights;
mod ga_config;
mod preview_config;
mod project_config;

pub use appendix_config::AppendixConfig;
pub use book_config::{BookConfig, CanvasConfig};
pub use book_layout_solver_config::{BookLayoutSolverConfig, ValidationError};
pub use cover_config::{CoverConfig, CoverMode, SpineConfig};
pub use fitness_weights::FitnessWeights;
pub use ga_config::GaConfig;
pub use preview_config::PreviewConfig;
pub use project_config::ProjectConfig;
