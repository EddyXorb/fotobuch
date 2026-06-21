//! Configuration structures for the YAML project state.
//!
//! This module contains the configuration structures that are persisted in `fotobuch.yaml`
//! and also used internally throughout the application to minimize translation overhead.

mod appendix_config;
mod book_config;
mod book_layout_solver_config;
mod book_layout_solver_validator;
mod canvas_config;
mod cover_config;
mod fitness_weights;
mod page_layout_solver_config;
mod preview_config;
mod project_config;

pub use appendix_config::AppendixConfig;
pub use book_config::BookConfig;
pub use book_layout_solver_config::{BookLayoutSolverConfig, ValidationError};
pub use book_layout_solver_validator::validate_book_layout_solver_config;
pub use canvas_config::CanvasConfig;
pub use cover_config::{CoverConfig, CoverMode, SpineConfig};
pub use fitness_weights::FitnessWeights;
pub use page_layout_solver_config::PageLayoutSolverConfig;
pub use preview_config::PreviewConfig;
pub use project_config::ProjectConfig;
