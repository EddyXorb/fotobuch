//! Handler for `fotobuch status` command

use anyhow::Context;
use anyhow::Result;
use photobook_solver::commands;

pub fn handle(page: Option<usize>) -> Result<()> {
    let project_root = std::env::current_dir()
        .context("Failed to determine current directory")?;

    let config = commands::StatusConfig { page };
    let report = commands::status(&project_root, &config)?;

    println!("Project: {}", report.project_name);
    println!(
        "{} photos in {} groups ({} unplaced)",
        report.total_photos, report.group_count, report.unplaced
    );
    println!();

    match report.state {
        commands::ProjectState_::Empty => {
            println!("Layout: empty (not yet built)");
        }
        commands::ProjectState_::Clean => {
            println!(
                "Layout: {} pages, {:.1} photos/page avg",
                report.page_count, report.avg_photos_per_page
            );
            println!("Status: clean (no changes since last build)");
        }
        commands::ProjectState_::Modified => {
            println!(
                "Layout: {} pages, {:.1} photos/page avg",
                report.page_count, report.avg_photos_per_page
            );
            println!(
                "Status: modified — {} page(s) changed since last build",
                report.page_changes.len()
            );
            if !report.page_changes.is_empty() {
                let pages_str = if report.page_changes.len() <= 5 {
                    report
                        .page_changes
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                } else {
                    format!(
                        "{}, ..., {}",
                        report.page_changes[0],
                        report.page_changes[report.page_changes.len() - 1]
                    )
                };
                println!("   pages: {}", pages_str);
            }
        }
    }

    if let Some(detail) = report.detail {
        println!();
        println!("Page {} — {} photos ({})", detail.page, detail.photo_count, if detail.modified { "modified" } else { "clean" });
        println!();

        for slot in &detail.slots {
            println!(
                "  {} — ratio {:.2} [{}]",
                slot.photo_id, slot.ratio, slot.swap_group
            );
            if slot.slot_mm != (0.0, 0.0, 0.0, 0.0) {
                println!(
                    "      slot: x={:.1}mm, y={:.1}mm, {}×{}mm",
                    slot.slot_mm.0, slot.slot_mm.1, slot.slot_mm.2, slot.slot_mm.3
                );
            }
        }
    }

    if !report.warnings.is_empty() {
        println!();
        println!("⚠️  Warnings:");
        for warning in &report.warnings {
            println!("   - {}", warning);
        }
    }

    Ok(())
}
