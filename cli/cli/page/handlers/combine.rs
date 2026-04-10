use anyhow::Result;

use fotobuch::commands::page;

use super::common::{format_page_list, project_root};
use crate::cli::page::parse_api::parse_pages_expr;

/// Handler for `fotobuch page combine <pages>`.
pub fn handle_combine(pages_str: &str) -> Result<()> {
    let pages = parse_pages_expr(pages_str)
        .map_err(|e| anyhow::anyhow!("Invalid pages expression '{}': {}", pages_str, e))?;
    let output =
        page::execute_combine(&project_root()?, pages).map_err(|e| anyhow::anyhow!("{}", e))?;
    println!(
        "Combined onto page {}. Deleted pages: {}",
        output.result.pages_modified.first().copied().unwrap_or(0),
        format_page_list(&output.result.pages_deleted)
    );
    Ok(())
}
