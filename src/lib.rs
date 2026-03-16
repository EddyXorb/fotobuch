//! Photobook Layout Solver
//!
//! Optimizes photo layouts for photobooks using genetic algorithms and mixed integer programming.
//!
//! ## Overview
//!
//! This library provides two optimization modes:
//!
//! * **Single-Page Optimization**: Uses a genetic algorithm with slicing trees to find optimal
//!   photo arrangements on a single page. Optimizes for size distribution, coverage, balance,
//!   and reading order.
//!
//! * **Multi-Page Optimization**: Uses Mixed Integer Programming (MIP) to assign photos to pages
//!   while respecting grouping constraints, followed by local search refinement. Each page is then
//!   individually optimized using the genetic algorithm approach.
//!
//! ## Example
//!
//! ```rust,no_run
//! use fotobuch::solver::{run_solver, Request, RequestType};
//! use fotobuch::dto_models::*;
//!
//! // Define book configuration
//! let book_config = BookConfig {
//!     title: "My Photobook".to_string(),
//!     page_width_mm: 297.0,
//!     page_height_mm: 210.0,
//!     bleed_mm: 3.0,
//!     margin_mm: 10.0,
//!     gap_mm: 5.0,
//!     bleed_threshold_mm: 3.0,
//!     dpi: 300.0,
//! };
//!
//! // Define solver parameters (use defaults for simplicity)
//! let solver_config = BookLayoutSolverConfig::default();
//!
//! // Define GA configuration
//! let ga_config = GaConfig::default();
//!
//! // Load photo groups (from directory or other source)
//! let photo_groups: Vec<PhotoGroup> = vec![/* ... */];
//!
//! // Create request
//! let request = Request {
//!     request_type: RequestType::MultiPage,  // or RequestType::SinglePage
//!     groups: &photo_groups,
//!     config: &solver_config,
//!     ga_config: &ga_config,
//!     book_config: &book_config,
//! };
//!
//! // Run solver
//! let layout_pages = run_solver(&request)
//!     .expect("Solver failed");
//!
//! // Process results
//! for page in layout_pages {
//!     println!("Page {}: {} photos", page.page, page.photos.len());
//! }
//! ```
//!
//! ## Algorithms
//!
//! ### Genetic Algorithm (Page Layout)
//!
//! The single-page layout uses a slicing tree representation evolved through genetic operators:
//! * **Crossover**: Exchanges subtrees between parent layouts
//! * **Mutation**: Randomly modifies tree structure (swap, rotate, change cut type)
//! * **Affine Solver**: Computes exact photo dimensions using O(N) dynamic programming
//!
//! Fitness combines multiple objectives: size distribution quality, page coverage,
//! barycenter position, and reading order coherence.
//!
//! ### Mixed Integer Programming (Page Assignment)
//!
//! The multi-page optimization first uses MIP to assign photos to pages:
//! * Respects photo group constraints (photos in same group stay together)
//! * Ensures page capacity constraints (min/max photos per page)
//! * Minimizes coverage deviation across pages
//! * Balances total aspect ratios per page
//!
//! The MIP solution is then refined using local search with moves like page swaps
//! and photo transfers, with each candidate evaluated using the GA-based page solver.
//!

pub mod cache;
pub mod commands;
pub mod dto_models;
pub mod git;
pub mod input;
pub mod output;
pub mod solver;
pub mod state_manager;

// Re-export core API types for convenience
pub use dto_models::{FitnessWeights, GaConfig};
pub use solver::run_solver;
