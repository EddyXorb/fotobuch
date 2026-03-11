pub(super) mod book_layout;
pub(super) mod canvas;
pub(super) mod layout;
pub(super) mod photo;
pub(super) mod test_fixtures;

pub use layout::{PhotoPlacement, SolverPageLayout};
pub use photo::Photo;
// Re-export all public types
pub use canvas::Canvas;
