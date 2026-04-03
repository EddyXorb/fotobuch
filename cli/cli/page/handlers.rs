//! CLI entry-point handlers for page and unplace subcommands.

use anyhow::{Context, Result};
use std::path::PathBuf;

use fotobuch::commands::page::{self as page_cmd, InfoFilter, PageMoveCmd, SlotInfo};
use fotobuch::commands::unplace::execute_unplace;
use fotobuch::dto_models::PageMode;

use super::parse_api::{
    parse_info_address, parse_move_cmd, parse_pages_expr, parse_pos_address, parse_split_addr,
    parse_swap_addrs, parse_unplace_addr, parse_weight_address,
};

fn project_root() -> Result<PathBuf> {
    std::env::current_dir().context("Failed to determine current directory")
}

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

/// Handler for `fotobuch page move <args...>`.
pub fn handle_move(args: &[String]) -> Result<()> {
    let raw = args.join(" ");
    let cmd = parse_move_cmd(&raw)
        .map_err(|e| anyhow::anyhow!("Invalid move expression '{}': {}", raw, e))?;
    let result =
        page_cmd::execute_move(&project_root()?, cmd).map_err(|e| anyhow::anyhow!("{}", e))?;
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
    let result =
        page_cmd::execute_combine(&project_root()?, pages).map_err(|e| anyhow::anyhow!("{}", e))?;
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
    let result =
        page_cmd::execute_move(&project_root()?, cmd).map_err(|e| anyhow::anyhow!("{}", e))?;
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

/// Handler for `fotobuch page info <address> [--weights|--ids|--pixels]`.
pub fn handle_info(address: &str, filter: InfoFilter) -> Result<()> {
    let addr = parse_info_address(address)
        .map_err(|e| anyhow::anyhow!("Invalid address '{}': {}", address, e))?;
    let result = page_cmd::execute_info(&project_root()?, addr, filter.clone())
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    if result.slots.is_empty() {
        println!("No slots found.");
        return Ok(());
    }

    if filter.weights {
        for s in &result.slots {
            println!("{}:{}={}", s.page, s.slot, s.area_weight);
        }
    } else if filter.ids {
        for s in &result.slots {
            println!("{}", s.id);
        }
    } else if filter.pixels {
        for s in &result.slots {
            println!("{}x{}", s.width_px, s.height_px);
        }
    } else if result.slots.len() == 1 {
        print_vertical(&result.slots[0]);
    } else {
        print_table(&result.slots);
    }
    Ok(())
}

fn print_vertical(s: &SlotInfo) {
    let ratio = s.width_px as f64 / s.height_px as f64;
    let page_label = if s.is_cover {
        format!("page {} [cover]", s.page)
    } else {
        format!("page {}", s.page)
    };
    println!("{page_label}, slot {}", s.slot);
    println!("  id:      {}", s.id);
    println!("  source:  {}", s.source);
    println!("  pixels:  {}x{}", s.width_px, s.height_px);
    println!("  ratio:   {ratio:.2}");
    println!("  weight:  {}", s.area_weight);
    println!(
        "  canvas:  {:.1}mm × {:.1}mm",
        s.page_width_mm, s.page_height_mm
    );
    if let Some(sl) = &s.placement {
        println!(
            "  placed:  x={:.1}mm y={:.1}mm w={:.1}mm h={:.1}mm",
            sl.x_mm, sl.y_mm, sl.width_mm, sl.height_mm
        );
    } else {
        println!("  placed:  (not yet placed)");
    }
}

fn print_table(slots: &[SlotInfo]) {
    struct Row {
        slot: String,
        ratio: String,
        weight: String,
        pixels: String,
        placed: String,
        id: String,
    }

    let rows: Vec<Row> = slots
        .iter()
        .map(|s| {
            let ratio = s.width_px as f64 / s.height_px as f64;
            let placed = s.placement.as_ref().map_or(String::new(), |sl| {
                format!(
                    "{:.1}, {:.1}, {:.1}x{:.1}",
                    sl.x_mm, sl.y_mm, sl.width_mm, sl.height_mm
                )
            });
            Row {
                slot: s.slot.to_string(),
                ratio: format!("{ratio:.2}"),
                weight: format!("{:.1}", s.area_weight),
                pixels: format!("{}x{}", s.width_px, s.height_px),
                placed,
                id: s.id.clone(),
            }
        })
        .collect();

    // Column widths: max of header and all row values.
    let w_slot = rows.iter().map(|r| r.slot.len()).max().unwrap_or(0).max(4);
    let w_ratio = rows.iter().map(|r| r.ratio.len()).max().unwrap_or(0).max(5);
    let w_weight = rows
        .iter()
        .map(|r| r.weight.len())
        .max()
        .unwrap_or(0)
        .max(6);
    let w_pixels = rows
        .iter()
        .map(|r| r.pixels.len())
        .max()
        .unwrap_or(0)
        .max(6);
    let w_placed = rows
        .iter()
        .map(|r| r.placed.len())
        .max()
        .unwrap_or(0)
        .max(6);

    let mut current_page: Option<u32> = None;
    for (s, row) in slots.iter().zip(rows.iter()) {
        if current_page != Some(s.page) {
            let shown = slots.iter().filter(|x| x.page == s.page).count();
            let cover_tag = if s.is_cover { " [cover]" } else { "" };
            let dims = format!("  {:.1}mm × {:.1}mm", s.page_width_mm, s.page_height_mm);
            if shown == s.total_page_slots {
                println!("page {}{cover_tag}{dims}", s.page);
            } else {
                println!(
                    "page {}{cover_tag}  ({}/{} slots shown){dims}",
                    s.page, shown, s.total_page_slots
                );
            }
            println!(
                "  {:<w_slot$}  {:<w_ratio$}  {:<w_weight$}  {:<w_pixels$}  {:<w_placed$}  id",
                "slot", "ratio", "weight", "pixels", "placed",
            );
            current_page = Some(s.page);
        }
        println!(
            "  {:<w_slot$}  {:<w_ratio$}  {:<w_weight$}  {:<w_pixels$}  {:<w_placed$}  {}",
            row.slot, row.ratio, row.weight, row.pixels, row.placed, row.id,
        );
    }
}

/// Handler for `fotobuch page weight <address> <weight>`.
pub fn handle_weight(address: &str, weight: f64) -> Result<()> {
    let addr = parse_weight_address(address)
        .map_err(|e| anyhow::anyhow!("Invalid address '{}': {}", address, e))?;
    page_cmd::execute_weight(&project_root()?, addr, weight)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    println!("Weight set to {weight}.");
    Ok(())
}

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

    let result = page_cmd::execute_mode(&project_root()?, pages, mode)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

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

/// Handler for `fotobuch page pos <address> [--by dx,dy] [--at x,y] [--scale s]`.
pub fn handle_pos(
    address: &str,
    by: Option<&str>,
    at: Option<&str>,
    scale: Option<f64>,
) -> Result<()> {
    use fotobuch::commands::page::{PosConfig, PosMode};

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
        // Was silent bug with splitn(2): "2,3,3" → ["2","3,3"] accepted "3,3" as y
        assert!(parse_mm_pair("2,3,3", "--by").is_err());
    }

    #[test]
    fn test_parse_mm_pair_not_a_number() {
        assert!(parse_mm_pair("a,3", "--by").is_err());
        assert!(parse_mm_pair("3,b", "--by").is_err());
    }
}
