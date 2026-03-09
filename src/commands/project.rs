//! Project management commands
//!
//! Handles creation, switching, and listing of photobook projects.
//! Each project lives on a separate `fotobuch/<name>` branch.

pub mod list;
pub mod new;

pub use list::project_list;
pub use new::{project_new, validate_project_name, NewConfig, NewResult};

/// Information about a project
#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub name: String,
    pub branch: String,
    pub is_current: bool,
}
