use anyhow::Result;

use fotobuch::commands::page::{self as page_cmd, InfoFilter, SlotInfo};

use super::common::project_root;
use crate::cli::page::parse_api::parse_info_address;

/// Handler for `fotobuch page info <address> [--weights|--ids|--pixels]`.
pub fn handle_info(address: &str, filter: InfoFilter) -> Result<()> {
    let addr = parse_info_address(address)
        .map_err(|e| anyhow::anyhow!("Invalid address '{}': {}", address, e))?;
    let result = page_cmd::execute_info(&project_root()?, addr, filter.clone())
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .result;

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
