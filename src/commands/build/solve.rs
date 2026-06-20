//! Solver engines: turn a set of photos into laid-out pages.
//!
//! This is the lowest layer of the build pipeline — input generation, the
//! `run_solver` call and applying the result to `state.layout`. The layout
//! builders one level up decide *which* photos and pages each engine receives.

pub(super) mod multi_page;
pub(super) mod single_page;
