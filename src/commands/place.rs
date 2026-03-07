//! `fotobuch place` command - Place unplaced photos into the book

use anyhow::Result;
use std::path::Path;

/// Configuration for placing photos
#[derive(Debug, Clone)]
pub struct PlaceConfig {
    /// Only place photos matching this pattern (optional)
    pub filter: Option<String>,
    /// Place all matching photos onto this page (optional)
    pub into_page: Option<usize>,
}

/// Result of placing photos
#[derive(Debug)]
pub struct PlaceResult {
    /// Number of photos placed
    pub photos_placed: usize,
    /// Pages affected by placements (need rebuild)
    pub pages_affected: Vec<usize>,
}

/// Place unplaced photos into the book
///
/// # Steps
/// 1. Find unplaced photos (in photos, not in layout)
/// 2. Apply filter if provided
/// 3. If into_page: place all matching photos onto that page
/// 4. Else: sort chronologically, insert into appropriate pages based on timestamp
/// 5. Update fotobuch.yaml (layout[].photos)
/// 6. Git commit: "place: N photos"
///
/// # Arguments
/// * `project_root` - Path to the project directory
/// * `config` - Configuration for placing photos
///
/// # Returns
/// * `PlaceResult` with count of placed photos and affected pages
pub fn place(project_root: &Path, config: &PlaceConfig) -> Result<PlaceResult> {
    // TODO: Implement photo placement
    // - Find unplaced photos
    // - Apply filter
    // - Sort chronologically (if not into_page)
    // - Insert into appropriate pages
    // - Update fotobuch.yaml
    // - Git commit

    let _ = (project_root, config); // Silence unused warnings

    Ok(PlaceResult {
        photos_placed: 0,
        pages_affected: Vec::new(),
    })
}
