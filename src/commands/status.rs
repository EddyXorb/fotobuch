//! `fotobuch status` command - Show project status

use anyhow::Result;
use std::path::Path;

/// Photo slot information for status display
#[derive(Debug)]
pub struct SlotInfo {
    /// Photo ID
    pub photo_id: String,
    /// Aspect ratio (width/height)
    pub ratio: f64,
    /// Layout slot dimensions in mm
    pub slot_mm: (f64, f64, f64, f64), // x, y, width, height
}

/// Page status information
#[derive(Debug)]
pub struct PageStatus {
    /// Page number (1-based)
    pub page: usize,
    /// Number of photos on this page
    pub photo_count: usize,
    /// Whether this page was modified since last build
    pub modified: bool,
    /// Detailed slot information (only for detail view)
    pub slots: Option<Vec<SlotInfo>>,
}

/// Overall project status
#[derive(Debug)]
pub struct StatusReport {
    /// Total number of photos in project
    pub total_photos: usize,
    /// Number of groups
    pub group_count: usize,
    /// Number of unplaced photos
    pub unplaced: usize,
    /// Total number of pages in layout
    pub page_count: usize,
    /// Average photos per page
    pub avg_photos_per_page: f64,
    /// Number of pages modified since last build
    pub modified_pages: usize,
    /// Detailed page information (empty for compact view, one entry for detail view)
    pub pages: Vec<PageStatus>,
    /// Warnings (orphaned placements, ratio mismatches, etc.)
    pub warnings: Vec<String>,
}

/// Show project status
///
/// # Steps
/// 1. Load fotobuch.yaml (current state)
/// 2. If page is None: show compact overview
///    - Total photos, groups, unplaced
///    - Layout summary (pages, avg photos/page)
///    - Modified pages count
/// 3. If page is Some(n): show detailed page view
///    - List all photos with slots, ratios
///    - Group photos by swap compatibility
///    - Mark page as modified/clean
/// 4. Run consistency checks:
///    - Unplaced photos (in photos, not in layout)
///    - Orphaned placements (in layout, not in photos)
///    - Aspect ratio mismatches after swaps
///
/// # Arguments
/// * `project_root` - Path to the project directory
/// * `page` - Optional page number for detailed view
///
/// # Returns
/// * `StatusReport` with project status and optional page details
pub fn status(project_root: &Path, page: Option<usize>) -> Result<StatusReport> {
    // TODO: Implement status command
    // - Load fotobuch.yaml
    // - Load last commit version (git show HEAD:fotobuch.yaml)
    // - Compare structs to find modified pages
    // - Run consistency checks
    // - Build status report

    let _ = (project_root, page); // Silence unused warnings

    Ok(StatusReport {
        total_photos: 0,
        group_count: 0,
        unplaced: 0,
        page_count: 0,
        avg_photos_per_page: 0.0,
        modified_pages: 0,
        pages: Vec::new(),
        warnings: Vec::new(),
    })
}
