//! Core solver for the photobook layout problem using slicing trees and genetic algorithms.
//!
//! This module contains:
//! - `solver`: High-level solver orchestration
//! - `page_layout`: Single-page layout optimization (tree, fitness, GA)

pub(crate) mod page_layout;
pub mod solver;

// Re-export commonly used types from page_layout
pub use page_layout::tree::{Cut, Node, SlicingTree};
pub use page_layout::layout_solver::solve_layout;
pub use page_layout::fitness::total_cost;
pub use page_layout::ga::{run_ga, GaConfig, IslandConfig};

// Re-export from solver
pub use solver::run_solver;

