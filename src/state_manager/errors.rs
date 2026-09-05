//! Typed errors of the state layer.
//!
//! The `#[error]` texts name only the *condition*; how to remedy it depends on
//! the consuming surface and is worded there.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("current branch '{branch}' is not a project branch (expected 'fotobuch/<name>')")]
    NotOnProjectBranch { branch: String },
}
