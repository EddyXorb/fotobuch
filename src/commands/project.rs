//! Project management commands
//!
//! Handles creation, switching, and listing of photobook projects.
//! Each project lives on a separate `fotobuch/<name>` branch.

pub mod errors;
pub mod list;
pub mod new;
pub mod switch;

pub use errors::ProjectError;
pub use list::list;
pub use new::{NewConfig, NewResult, new, validate_project_name};
pub use switch::switch;

/// Information about a project
#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub name: String,
    pub branch: String,
    pub is_current: bool,
}
