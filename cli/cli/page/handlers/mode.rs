use anyhow::Result;

use fotobuch::commands::page;
use fotobuch::dto_models::PageMode;

use super::common::{format_page_list, project_root};
use crate::cli::page::parse_api::parse_pages_expr;

/// Handler for `fotobuch page mode <pages> <mode>`.
pub fn handle_mode(pages_str: &str, mode_str: &str) -> Result<()> {
    let pages = parse_pages_expr(pages_str)
        .map_err(|e| anyhow::anyhow!("Invalid pages expression '{}': {}", pages_str, e))?;

    let mode = match mode_str {
        "a" | "auto" => PageMode::Auto,
        "m" | "manual" => PageMode::Manual,
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid mode '{}'. Use 'a', 'm', 'auto', or 'manual'.",
                mode_str
            ));
        }
    };

    let result =
        page::execute_mode(&project_root()?, pages, mode).map_err(|e| anyhow::anyhow!("{}", e))?;

    let mode_name = match result.new_mode {
        PageMode::Auto => "auto",
        PageMode::Manual => "manual",
    };

    println!(
        "Set {} page(s) to {} mode: {}",
        result.pages_changed.len(),
        mode_name,
        format_page_list(&result.pages_changed)
    );
    Ok(())
}
