use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("project '{name}' not found")]
    NotFound { name: String },
    #[error("an active cover requires a spine configuration (growth per 10 pages or fixed width)")]
    CoverWithoutSpine,
}
