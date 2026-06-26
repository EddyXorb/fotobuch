use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("project '{name}' not found")]
    NotFound { name: String },
}