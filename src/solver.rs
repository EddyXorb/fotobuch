//! Core solver for the photobook layout problem using slicing trees and genetic algorithms.
//!
//! This module contains:
//! - `solver`: High-level solver orchestration
//! - `tree`: Slicing tree data structure (arena-based)
//! - `layout_solver`: Affine layout solver (O(N) with β support)
//! - `fitness`: Fitness function components
//! - `ga`: Genetic algorithm main loop

pub mod solver;
pub mod tree;
pub mod layout_solver;
pub mod fitness;
pub mod ga;

// Re-export commonly used types
pub use tree::{Cut, Node, SlicingTree};
pub use layout_solver::solve_layout;
pub use fitness::total_cost;
pub use ga::{run_ga, run_island_ga, GaConfig, IslandConfig};
pub use solver::run_solver;

