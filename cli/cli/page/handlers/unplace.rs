use anyhow::Result;

use fotobuch::commands::unplace::execute_unplace;

use super::common::project_root;
use crate::cli::page::parse_api::parse_unplace_addr;

/// Handler for `fotobuch unplace <address>`.
pub fn handle_unplace(address: &str) -> Result<()> {
    let (page, slots) = parse_unplace_addr(address)
        .map_err(|e| anyhow::anyhow!("Invalid address '{}': {}", address, e))?;
    let result =
        execute_unplace(&project_root()?, page, slots).map_err(|e| anyhow::anyhow!("{}", e))?;
    if result.pages_modified.is_empty() {
        println!("Nothing to unplace.");
    } else {
        println!("Unplaced photos from page {}.", page);
    }
    Ok(())
}
