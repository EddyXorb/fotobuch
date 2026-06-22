//! `fotobuch place` command - Place unplaced photos into the book

mod page_placement;
mod placement;
mod selection;

use anyhow::Result;
use std::path::Path;

use crate::commands::page::format_pages_list;
use crate::commands::{CommandOutput, run_write_command};
use crate::models::ProjectState;

/// Target destination for placing photos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceDst {
    /// Place chronologically across existing pages (default).
    Auto,
    /// Place onto an existing page (0-based index).
    Page(usize),
    /// Create a new page at this position and place photos there.
    NewPageAt(usize),
}

/// Configuration for placing photos
#[derive(Debug, Clone)]
pub struct PlaceConfig {
    /// Only place photos matching these patterns (all must match)
    pub filters: Vec<String>,
    /// Restrict to these photo IDs (empty = no restriction)
    pub ids: Vec<String>,
    /// Where to place the photos
    pub dst: PlaceDst,
}

/// Result of placing photos
#[derive(Debug)]
pub struct PlaceResult {
    /// Number of photos placed
    pub photos_placed: usize,
    /// Pages affected by placements (need rebuild)
    pub pages_affected: Vec<usize>,
    /// Pages newly inserted (0-based positions after insertion)
    pub pages_inserted: Vec<usize>,
}

fn empty_result() -> PlaceResult {
    PlaceResult {
        photos_placed: 0,
        pages_affected: vec![],
        pages_inserted: vec![],
    }
}

/// Place unplaced photos into the book.
///
/// Flow: validate destination → find unplaced photos → filter (regex + ids) →
/// write into the layout → commit `place: N photos onto <pages>`.
pub fn place(project_root: &Path, config: &PlaceConfig) -> Result<CommandOutput<PlaceResult>> {
    run_write_command(project_root, |mgr| {
        validate_destination(mgr.state(), &config.dst)?;

        let unplaced = selection::find_unplaced(mgr.state());
        if unplaced.is_empty() {
            return Ok((String::new(), empty_result()));
        }

        let after_regex = selection::apply_filters(&unplaced, &config.filters)?;
        let filtered: Vec<_> = if config.ids.is_empty() {
            after_regex
        } else {
            after_regex
                .into_iter()
                .filter(|p| config.ids.contains(&p.id))
                .collect()
        };
        if filtered.is_empty() {
            return Ok((String::new(), empty_result()));
        }

        let mut view = mgr.get_write_layout_state();
        let (pages_affected, pages_inserted) =
            placement::place_photos(&mut view, &config.dst, &filtered);

        let photos_placed = filtered.len();
        let pages_u32: Vec<u32> = pages_affected.iter().map(|&p| p as u32).collect();
        let pages_str = format_pages_list(&pages_u32);

        Ok((
            format!("place: {photos_placed} photos onto {pages_str}"),
            PlaceResult {
                photos_placed,
                pages_affected,
                pages_inserted,
            },
        ))
    })
}

/// Reject destinations that cannot apply to the current layout.
fn validate_destination(state: &ProjectState, dst: &PlaceDst) -> Result<()> {
    if state.layout.is_empty() && !matches!(dst, PlaceDst::NewPageAt(_)) {
        anyhow::bail!("No layout yet. Run `fotobuch build` first.");
    }
    match dst {
        PlaceDst::Page(page) if *page >= state.layout.len() => {
            anyhow::bail!(
                "Invalid page {} (layout has {} pages, indices 0..{})",
                page,
                state.layout.len(),
                state.layout.len().saturating_sub(1),
            );
        }
        PlaceDst::NewPageAt(pos) if *pos > state.layout.len() => {
            anyhow::bail!(
                "Invalid new page position {} (layout has {} pages, valid range 0..={})",
                pos,
                state.layout.len(),
                state.layout.len(),
            );
        }
        _ => {}
    }
    Ok(())
}
