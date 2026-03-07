//! Configuration structures for the YAML project state.
//!
//! This module contains the configuration structures that are persisted in `fotobuch.yaml`
//! and also used internally throughout the application to minimize translation overhead.

mod book_config;
mod fitness_weights;
mod ga_config;
mod preview_config;
mod project_config;

pub use book_config::BookConfig;
pub use fitness_weights::FitnessWeights;
pub use ga_config::GaConfig;
pub use preview_config::PreviewConfig;
pub use project_config::ProjectConfig;
