use anyhow::{Context, Result};
use std::path::PathBuf;

pub fn project_root() -> Result<PathBuf> {
    std::env::current_dir().context("Failed to determine current directory")
}

pub fn format_page_list(pages: &[u32]) -> String {
    pages
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
