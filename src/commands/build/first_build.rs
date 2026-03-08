use super::super::BuildResult;
use crate::cache::preview;
use crate::output::typst;
use crate::solver::{Request, RequestType, run_solver};
use crate::state_manager::StateManager;
use anyhow::Result;
use std::{path::Path, sync::atomic::AtomicUsize};

/// Performs the first build: generates layout for all photos and creates preview PDF.
pub fn first_build(mut mgr: StateManager, project_root: &Path) -> Result<BuildResult> {
    println!("First build: creating layout for all photos...");

    // 1. Generate preview cache for all photos
    let progress = AtomicUsize::new(0);
    let preview_cache_dir = mgr.preview_cache_dir();
    let cache_result = preview::ensure_previews(&mgr.state, &preview_cache_dir, &progress)?;

    println!(
        "Preview cache: {} created, {} skipped, {} total",
        cache_result.created, cache_result.skipped, cache_result.total
    );

    // 2. Run MultiPage solver to distribute photos across pages
    let request = Request {
        request_type: RequestType::MultiPage,
        groups: &mgr.state.photos,
        config: &mgr.state.config.book_layout_solver,
        ga_config: &mgr.state.config.page_layout_solver,
        book_config: &mgr.state.config.book,
    };

    let pages = run_solver(&request)?;
    let total_cost = 0.0; //TODO: get actual cost from solver result when available

    println!("Solver: generated {} pages", pages.len());

    // 3. Update state with layout
    mgr.state.layout = pages;

    // 4. Compile Typst template to PDF
    let pdf_path = typst::compile_preview(project_root, mgr.project_name())?;
    println!("PDF generated: {}", pdf_path.display());

    // 5. Save state and commit
    let pages_rebuilt: Vec<usize> = (1..=mgr.state.layout.len()).collect();
    let page_count = mgr.state.layout.len();

    mgr.finish(&format!(
        "build: {} pages (cost: {:.4})",
        page_count, total_cost
    ))?;

    Ok(BuildResult {
        pdf_path,
        pages_rebuilt,
        pages_swapped: vec![],
        images_processed: cache_result.created,
        total_cost,
        dpi_warnings: vec![],
        nothing_to_do: false,
    })
}
