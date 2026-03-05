//! Single-page layout optimization using slicing trees.
//!
//! This module contains the core components for single-page layout:
//! - `tree`: Slicing tree data structure
//! - `layout_solver`: Affine layout solver (O(N) with β support)
//! - `fitness`: Fitness function components

pub(super) mod tree;
pub(super) mod layout_solver;
pub(super) mod fitness;
