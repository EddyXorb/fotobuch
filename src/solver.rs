//! Core solver for the photobook layout problem using slicing trees and genetic algorithms.
//!
//! This module contains:
//! - `solver`: High-level solver orchestration
//! - `ga_solver`: Generic genetic algorithm implementation
//! - `page_layout_solver`: Single-page layout optimization (tree, fitness)
//! - `book_layout_solver`: Multi-page book layout optimization

pub(crate) mod book_layout_solver;
pub(crate) mod ga_solver;
pub(crate) mod page_layout_solver;
#[allow(clippy::module_inception)]
pub mod solver;

// Re-export from solver
pub use solver::run_solver;
