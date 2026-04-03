use anyhow::Result;

use fotobuch::commands::page;

use super::common::project_root;
use crate::cli::page::parse_api::parse_weight_address;

/// Handler for `fotobuch page weight <address> <weight>`.
pub fn handle_weight(address: &str, weight: f64) -> Result<()> {
    let addr = parse_weight_address(address)
        .map_err(|e| anyhow::anyhow!("Invalid address '{}': {}", address, e))?;
    page::execute_weight(&project_root()?, addr, weight).map_err(|e| anyhow::anyhow!("{}", e))?;
    println!("Weight set to {weight}.");
    Ok(())
}
