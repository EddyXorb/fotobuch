//! Core solver for the photobook layout problem using slicing trees and genetic algorithms.
//!
//! This module contains:
//! - `solver`: High-level solver orchestration
//! - `page_layout`: Single-page layout optimization (tree, fitness, GA)

pub(crate) mod page_layout;
pub mod solver;

// Re-export from solver
pub use solver::run_solver;
