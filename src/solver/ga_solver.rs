//! Generic genetic algorithm framework.
//!
//! This module provides a domain-agnostic genetic algorithm implementation
//! with support for parallel island-based evolution.
//!
//! # Architecture
//! - **solver**: Main GeneticAlgorithm orchestrator
//! - **individual**: Individual trait for population members
//! - **evolution**: EvolutionDynamic trait + Island/World structures
//! - **config**: Configuration parameters
//!
//! # Zero Dependencies
//! This module has NO dependencies on domain-specific code (photos, layouts, etc.).
//! It provides only the generic GA framework.

pub mod solver;
pub mod individual;
pub mod evolution;
pub mod config;

// Re-export main types
pub use solver::GeneticAlgorithm;
pub use individual::Individual;
pub use evolution::EvolutionDynamic;
pub use config::Config;

