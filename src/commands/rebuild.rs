//! `fotobuch rebuild` command - Force re-optimization of pages

use anyhow::Result;
use std::path::Path;

use super::build::BuildResult;

/// Scope of rebuild operation
#[derive(Debug, Clone)]
pub enum RebuildScope {
    /// Rebuild all pages (like first build)
    All,
    /// Rebuild single page (forced, even if clean)
    SinglePage(usize),
    /// Rebuild page range with optional flexibility
    Range {
        /// Start page (inclusive)
        start: usize,
        /// End page (inclusive)
        end: usize,
        /// Allow page count to vary by +/- N (default: 0)
        flex: usize,
    },
}

/// Force re-optimization of pages or page ranges
///
/// # Behavior by scope:
///
/// ## Single page: `rebuild 5`
/// - Page-Layout-Solver on page 5, forced even if clean
/// - Photo assignment stays the same, only layout[5].slots is rewritten
/// - Does not trigger Book-Layout-Solver
///
/// ## Page range: `rebuild 3-7`
/// - Book-Layout-Solver on subset: redistribute photos from pages 3-7
/// - Then Page-Layout-Solver for each page in that range
/// - Surrounding pages unchanged
/// - Page count stays the same (5 pages in, 5 pages out) unless --flex is used
///
/// ## With flex: `rebuild 3-7 --flex 2`
/// - Same as range, but solver may use 3-9 pages instead of exactly 5
/// - Useful after `place` when photos are unevenly distributed
///
/// ## All: `rebuild` (no arguments)
/// - Like first build: all photos from photos (top-level), fresh distribution
/// - Book-Layout-Solver + Page-Layout-Solver for all pages
/// - Manual changes in layout are lost (but git-recoverable)
///
/// # Steps
/// 1. Git pre-commit: "pre-rebuild: page 5" / "pre-rebuild: pages 3-7" / "pre-rebuild: all"
/// 2. Preview cache check
/// 3. Run appropriate solver(s)
/// 4. Write fotobuch.yaml
/// 5. Compile Typst -> PDF
/// 6. Git post-commit: "post-rebuild: page 5 (cost: X)"
///
/// # Arguments
/// * `project_root` - Path to the project directory
/// * `scope` - Rebuild scope (all, single page, or range)
///
/// # Returns
/// * `BuildResult` with PDF path and statistics
pub fn rebuild(project_root: &Path, scope: RebuildScope) -> Result<BuildResult> {
    // TODO: Implement rebuild command
    // - Git pre-commit with scope
    // - Determine which solver(s) to run
    // - Single page: Page-Layout-Solver only
    // - Range: Book-Layout-Solver on subset + Page-Layout-Solver for each
    // - All: Full Book-Layout + Page-Layout for all
    // - Write fotobuch.yaml
    // - Compile Typst
    // - Git post-commit with cost

    let _ = (project_root, scope); // Silence unused warnings

    Ok(BuildResult {
        pdf_path: std::path::PathBuf::from("fotobuch_preview.pdf"),
        pages_rebuilt: vec![],
        pages_swapped: vec![],
        images_processed: 0,
        total_cost: 0.0,
        dpi_warnings: Vec::new(),
        nothing_to_do: false,
    })
}
