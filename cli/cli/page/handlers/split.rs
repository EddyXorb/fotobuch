use anyhow::Result;

use fotobuch::commands::page;

use super::common::project_root;
use crate::cli::page::parse_api::parse_split_addr;

/// Handler for `fotobuch page split <address>`.
pub fn handle_split(address: &str) -> Result<()> {
    let (page, slot) = parse_split_addr(address)
        .map_err(|e| anyhow::anyhow!("Invalid split address '{}': {}", address, e))?;
    let output =
        page::execute_split(&project_root()?, page, slot).map_err(|e| anyhow::anyhow!("{}", e))?;
    println!(
        "Split page {} at slot {}. New page inserted after page {}.",
        page,
        slot,
        output.result.pages_inserted.first().copied().unwrap_or(0)
    );
    Ok(())
}
