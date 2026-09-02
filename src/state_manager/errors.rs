//! Typed errors of the state layer.
//!
//! The `#[error]` texts name only the *condition*; the remediation hint is
//! added by the surface (see `cli/cli/hints.rs`).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("current branch '{branch}' is not a project branch (expected 'fotobuch/<name>')")]
    NotOnProjectBranch { branch: String },
}
