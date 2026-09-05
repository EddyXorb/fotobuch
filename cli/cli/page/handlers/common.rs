use anyhow::{Context, Result};
use fotobuch::commands::page::PageMoveError;
use std::path::PathBuf;

pub fn project_root() -> Result<PathBuf> {
    std::env::current_dir().context("Failed to determine current directory")
}

/// Convert a page command error into an `anyhow::Error` without losing type
/// information: a validation error stays downcastable so `hints::hint_for` can
/// attach the remediation, and a wrapped error is unwrapped so its own cause
/// chain survives.
pub fn to_anyhow(err: PageMoveError) -> anyhow::Error {
    match err {
        PageMoveError::Other(inner) => inner,
        validation => anyhow::Error::new(validation),
    }
}

pub fn format_page_list(pages: &[u32]) -> String {
    pages
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
