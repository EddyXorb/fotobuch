//! `fotobuch page combine` command.

use std::path::Path;

use crate::state_manager::StateManager;

use super::helpers::{format_pages_list, page_idx};
use super::types::{PageMoveError, PageMoveResult, PagesExpr, ValidationError};

/// Combine all given pages onto the first page and delete the rest.
///
/// Pages in `pages_expr` must be 0-based. At least two pages required.
pub fn execute_combine(
    project_root: &Path,
    pages_expr: PagesExpr,
) -> Result<PageMoveResult, PageMoveError> {
    let mut mgr = StateManager::open(project_root)?;

    if pages_expr.pages.len() < 2 {
        let p = pages_expr.pages.first().copied().unwrap_or(0);
        return Err(ValidationError::CombineSinglePage(p).into());
    }

    for &p in &pages_expr.pages {
        page_idx(p, &mgr.state.layout)?;
    }

    let first_page = pages_expr.pages[0];
    let first_idx = page_idx(first_page, &mgr.state.layout)?;

    let mut extra_photos: Vec<String> = Vec::new();
    let other_pages: Vec<u32> = pages_expr.pages[1..].to_vec();
    for &p in &other_pages {
        let idx = page_idx(p, &mgr.state.layout)?;
        extra_photos.extend(mgr.state.layout[idx].photos.clone());
    }

    mgr.state.layout[first_idx].photos.extend(extra_photos);
    mgr.state.layout[first_idx].slots.clear(); // needs rebuild

    let mut delete_indices: Vec<usize> = other_pages
        .iter()
        .map(|&p| page_idx(p, &mgr.state.layout).unwrap())
        .collect();
    delete_indices.sort_unstable_by(|a, b| b.cmp(a));
    for idx in &delete_indices {
        mgr.state.layout.remove(*idx);
    }

    let pages_str = format_pages_list(&pages_expr.pages);
    mgr.finish(&format!("page combine: {pages_str}"))?;

    Ok(PageMoveResult {
        pages_modified: vec![first_page],
        pages_inserted: vec![],
        pages_deleted: other_pages,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::{make_state_with_layout, setup_repo};
    use super::super::types::{PagesExpr, ValidationError};
    use super::*;
    use crate::state_manager::StateManager;
    use tempfile::TempDir;

    #[test]
    fn test_execute_combine_merges_pages() {
        let state = make_state_with_layout(vec![
            vec!["p0.jpg", "p1.jpg"],
            vec!["p2.jpg"],
            vec!["p3.jpg", "p4.jpg"],
        ]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let pages = PagesExpr::from_range(0, 2);
        let result = execute_combine(tmp.path(), pages).unwrap();
        assert_eq!(result.pages_deleted, vec![1, 2]);

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state.layout.len(), 1);
        assert_eq!(mgr.state.layout[0].photos.len(), 5);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_combine_single_page_is_error() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let pages = PagesExpr::single(0);
        let result = execute_combine(tmp.path(), pages);
        assert!(matches!(
            result,
            Err(PageMoveError::Validation(
                ValidationError::CombineSinglePage(0)
            ))
        ));
    }
}
