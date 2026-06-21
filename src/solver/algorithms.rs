//! Generic algorithm implementations used by the solver.
//!
//! Infrastructure modules — no domain knowledge (photos, layouts, etc.):
//! - `genetic_algorithm`: Domain-agnostic genetic algorithm framework
//! - `bellman_dp`: Generic dynamic-programming solver (Bellman backward induction)

pub(crate) mod bellman_dp;
pub(crate) mod genetic_algorithm;
