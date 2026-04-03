use anyhow::Result;

use fotobuch::commands::page::{self as page_cmd, PosConfig, PosMode};

use super::common::project_root;
use crate::cli::page::parse_api::parse_pos_address;

/// Handler for `fotobuch page pos <address> [--by dx,dy] [--at x,y] [--scale s]`.
pub fn handle_pos(
    address: &str,
    by: Option<&str>,
    at: Option<&str>,
    scale: Option<f64>,
) -> Result<()> {
    let (page, slots) = parse_pos_address(address)
        .map_err(|e| anyhow::anyhow!("Invalid address '{}': {}", address, e))?;

    let position = if let Some(by_str) = by {
        let (dx, dy) = parse_mm_pair(by_str, "--by")?;
        Some(PosMode::Relative {
            dx_mm: dx,
            dy_mm: dy,
        })
    } else if let Some(at_str) = at {
        let (x, y) = parse_mm_pair(at_str, "--at")?;
        Some(PosMode::Absolute { x_mm: x, y_mm: y })
    } else {
        None
    };

    let config = PosConfig { position, scale };

    let result = page_cmd::execute_pos(&project_root()?, page, slots, &config)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    println!(
        "Moved {} slot(s) on page {}.",
        result.slots_changed.len(),
        result.page,
    );
    Ok(())
}

/// Parse a `"value,value"` pair of mm coordinates.
fn parse_mm_pair(raw: &str, flag: &str) -> Result<(f64, f64)> {
    let parts: Vec<&str> = raw.splitn(3, ',').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!(
            "Invalid {} value '{}': expected 'number,number' (e.g. '-20,30')",
            flag,
            raw
        ));
    }
    let parse_num = |s: &str| {
        s.trim().parse::<f64>().map_err(|_| {
            anyhow::anyhow!("Invalid {} value '{}': '{}' is not a number", flag, raw, s)
        })
    };
    Ok((parse_num(parts[0])?, parse_num(parts[1])?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mm_pair_valid() {
        assert_eq!(parse_mm_pair("-20,30", "--by").unwrap(), (-20.0, 30.0));
    }

    #[test]
    fn test_parse_mm_pair_too_few_parts() {
        assert!(parse_mm_pair("42", "--by").is_err());
    }

    #[test]
    fn test_parse_mm_pair_too_many_parts() {
        assert!(parse_mm_pair("2,3,3", "--by").is_err());
    }

    #[test]
    fn test_parse_mm_pair_not_a_number() {
        assert!(parse_mm_pair("a,3", "--by").is_err());
        assert!(parse_mm_pair("3,b", "--by").is_err());
    }
}
