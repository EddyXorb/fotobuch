//! Generic genetic algorithm framework.
//!
//! Domain-agnostic GA implementation with parallel island-based evolution.
//!
//! # Architecture
//! - **engine**: Main GeneticAlgorithm orchestrator
//! - **individual**: Individual trait for population members
//! - **evolution**: EvolutionDynamic trait + Island/World structures
//! - **config**: Configuration parameters
//!
//! No dependencies on domain-specific code (photos, layouts, etc.).

pub mod config;
pub mod engine;
pub mod evolution;
pub mod individual;

// Re-export main types
pub use config::Config;
pub use engine::GeneticAlgorithm;
pub use evolution::EvolutionDynamic;
pub use individual::Individual;
