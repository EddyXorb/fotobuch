//! Single-page layout optimization using slicing trees and genetic algorithms.
//!
//! This module contains the core components for optimizing photo placement:
//! - `tree`: Slicing tree data structure
//! - `layout_solver`: Affine layout solver (O(N) with β support)
//! - `fitness`: Fitness function components
//! - `ga`: Genetic algorithm

pub(super) mod tree;
pub(super) mod layout_solver;
pub(super) mod fitness;
pub(super) mod ga;
