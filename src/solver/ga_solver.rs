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

pub mod config;
pub mod evolution;
pub mod individual;
pub mod solver;

// Re-export main types
pub use config::Config;
pub use evolution::EvolutionDynamic;
pub use individual::Individual;
pub use solver::GeneticAlgorithm;
