//! CLI entry-point handlers for page and unplace subcommands.

use anyhow::{Context, Result};
use std::path::PathBuf;

use fotobuch::commands::page::{self as page_cmd, PageMoveCmd};
use fotobuch::commands::unplace::execute_unplace;

use super::parse_api::{
    parse_move_cmd, parse_pages_expr, parse_split_addr, parse_swap_addrs, parse_unplace_addr,
};

fn project_root() -> Result<PathBuf> {
    std::env::current_dir().context("Failed to determine current directory")
}

/// Handler for `fotobuch unplace <address>`.
pub fn handle_unplace(address: &str) -> Result<()> {
    let (page, slots) = parse_unplace_addr(address)
        .map_err(|e| anyhow::anyhow!("Invalid address '{}': {}", address, e))?;
    let result = execute_unplace(&project_root()?, page, slots)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    if result.pages_modified.is_empty() {
        println!("Nothing to unplace.");
    } else {
        println!("Unplaced photos from page {}.", page);
    }
    Ok(())
}

/// Handler for `fotobuch page move <args...>`.
pub fn handle_move(args: &[String]) -> Result<()> {
    let raw = args.join(" ");
    let cmd = parse_move_cmd(&raw)
        .map_err(|e| anyhow::anyhow!("Invalid move expression '{}': {}", raw, e))?;
    let result = page_cmd::execute_move(&project_root()?, cmd)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    if result.pages_deleted.is_empty() {
        println!(
            "Moved photos. Modified pages: {}",
            format_page_list(&result.pages_modified)
        );
        if !result.pages_inserted.is_empty() {
            println!(
                "Inserted new pages: {}",
                format_page_list(&result.pages_inserted)
            );
        }
    } else {
        if !result.pages_modified.is_empty() {
            println!(
                "Unplaced slots from page(s): {}",
                format_page_list(&result.pages_modified)
            );
        }
        println!(
            "Unplaced and deleted page(s): {}",
            format_page_list(&result.pages_deleted)
        );
    }
    Ok(())
}

/// Handler for `fotobuch page split <address>`.
pub fn handle_split(address: &str) -> Result<()> {
    let (page, slot) = parse_split_addr(address)
        .map_err(|e| anyhow::anyhow!("Invalid split address '{}': {}", address, e))?;
    let result = page_cmd::execute_split(&project_root()?, page, slot)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    println!(
        "Split page {} at slot {}. New page inserted after page {}.",
        page,
        slot,
        result.pages_inserted.first().copied().unwrap_or(0)
    );
    Ok(())
}

/// Handler for `fotobuch page combine <pages>`.
pub fn handle_combine(pages_str: &str) -> Result<()> {
    let pages = parse_pages_expr(pages_str)
        .map_err(|e| anyhow::anyhow!("Invalid pages expression '{}': {}", pages_str, e))?;
    let result = page_cmd::execute_combine(&project_root()?, pages)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    println!(
        "Combined onto page {}. Deleted pages: {}",
        result.pages_modified.first().copied().unwrap_or(0),
        format_page_list(&result.pages_deleted)
    );
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
    let result = page_cmd::execute_move(&project_root()?, cmd)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    println!(
        "Swapped photos. Modified pages: {}",
        format_page_list(&result.pages_modified)
    );
    Ok(())
}

pub fn format_page_list(pages: &[u32]) -> String {
    let list: Vec<String> = pages.iter().map(|p| p.to_string()).collect();
    list.join(", ")
}
