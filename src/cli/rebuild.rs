//! Handler for `fotobuch rebuild` command

use anyhow::Context;
use anyhow::Result;
use photobook_solver::commands;

pub fn handle(
    page: Option<usize>,
    range_start: Option<usize>,
    range_end: Option<usize>,
    flex: usize,
    all: bool,
) -> Result<()> {
    let project_root = std::env::current_dir()
        .context("Failed to determine current directory")?;

    let scope = if all {
        commands::rebuild::RebuildScope::All
    } else if let Some(p) = page {
        commands::rebuild::RebuildScope::SinglePage(p)
    } else if let (Some(start), Some(end)) = (range_start, range_end) {
        commands::rebuild::RebuildScope::Range {
            start,
            end,
            flex,
        }
    } else {
        commands::rebuild::RebuildScope::All
    };

    let result = commands::rebuild::rebuild(&project_root, scope)?;

    if !result.pages_rebuilt.is_empty() {
        println!("✅ Rebuilt {} page(s): {:?}",
            result.pages_rebuilt.len(),
            result.pages_rebuilt
        );
    }
    println!("📄 PDF: {}", result.pdf_path.display());

    Ok(())
}
