pub(super) mod book_layout;
pub(super) mod canvas;
pub(super) mod layout;
pub(super) mod photo;
pub(super) mod request;
pub(super) mod test_fixtures;

pub use layout::{SolverPageLayout, PhotoPlacement};
pub use photo::Photo;
// Re-export all public types
pub use canvas::Canvas;
