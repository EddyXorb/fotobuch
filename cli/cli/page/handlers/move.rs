use anyhow::Result;

use fotobuch::commands::page::{self as page_cmd, PageMoveCmd};

use super::common::{format_page_list, project_root};
use crate::cli::page::parse_api::{parse_move_cmd, parse_swap_addrs};

/// Handler for `fotobuch page move <args...>`.
pub fn handle_move(args: &[String]) -> Result<()> {
    let raw = args.join(" ");
    let cmd = parse_move_cmd(&raw)
        .map_err(|e| anyhow::anyhow!("Invalid move expression '{}': {}", raw, e))?;
    let output =
        page_cmd::execute_move(&project_root()?, cmd).map_err(|e| anyhow::anyhow!("{}", e))?;
    if output.result.pages_deleted.is_empty() {
        println!(
            "Moved photos. Modified pages: {}",
            format_page_list(&output.result.pages_modified)
        );
        if !output.result.pages_inserted.is_empty() {
            println!(
                "Inserted new pages: {}",
                format_page_list(&output.result.pages_inserted)
            );
        }
    } else {
        if !output.result.pages_modified.is_empty() {
            println!(
                "Unplaced slots from page(s): {}",
                format_page_list(&output.result.pages_modified)
            );
        }
        println!(
            "Unplaced and deleted page(s): {}",
            format_page_list(&output.result.pages_deleted)
        );
    }
    Ok(())
}

/// Handler for `fotobuch page swap <left> <right>`.
pub fn handle_swap(left: &str, right: &str) -> Result<()> {
    let (left_src, right_dst) = parse_swap_addrs(left, right)
        .map_err(|e| anyhow::anyhow!("Invalid swap addresses '{}' '{}': {}", left, right, e))?;
    let cmd = PageMoveCmd::Swap {
        left: left_src,
        right: right_dst,
    };
    let output =
        page_cmd::execute_move(&project_root()?, cmd).map_err(|e| anyhow::anyhow!("{}", e))?;
    println!(
        "Swapped photos. Modified pages: {}",
        format_page_list(&output.result.pages_modified)
    );
    Ok(())
}
