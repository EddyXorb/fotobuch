use thiserror::Error;

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("no layout exists")]
    NoLayout,
    #[error("layout has changes since last build; changed pages: {pages:?}")]
    LayoutDirty { pages: Vec<usize> },
    #[error("page {idx} is in manual mode")]
    PageIsManual { idx: usize },
    #[error("range contains only the cover page (index 0)")]
    CoverExcluded,
}