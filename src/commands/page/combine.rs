//! `fotobuch page combine` command.

use std::path::Path;

use crate::commands::{CommandOutput, run_write_command};
use crate::state_manager::{ReadOnlyState, WriteLayoutState};

use super::helpers::{format_pages_list, page_idx};
use super::types::{PageMoveError, PageMoveResult, PagesExpr, ValidationError};

/// Combine all given pages onto the first page and delete the rest.
///
/// Pages in `pages_expr` must be 0-based. At least two pages required.
pub fn execute_combine(
    project_root: &Path,
    pages_expr: PagesExpr,
) -> Result<CommandOutput<PageMoveResult>, PageMoveError> {
    run_write_command(project_root, |mgr| {
        if pages_expr.pages.len() < 2 {
            let p = pages_expr.pages.first().copied().unwrap_or(0);
            return Err(ValidationError::CombineSinglePage(p).into());
        }

        let first_page = pages_expr.pages[0];
        let other_pages: Vec<u32> = pages_expr.pages[1..].to_vec();

        let mut view = mgr.get_write_layout_state();
        combine_onto_first(&mut view, &pages_expr.pages, first_page, &other_pages)?;

        let pages_str = format_pages_list(&pages_expr.pages);
        Ok((
            format!("page combine: {pages_str}"),
            PageMoveResult {
                pages_modified: vec![first_page],
                pages_inserted: vec![],
                pages_deleted: other_pages,
            },
        ))
    })
}

/// Merge the `other_pages`' photos onto `first_page` and delete the merged pages.
fn combine_onto_first(
    s: &mut WriteLayoutState,
    all_pages: &[u32],
    first_page: u32,
    other_pages: &[u32],
) -> Result<(), ValidationError> {
    for &p in all_pages {
        page_idx(p, s.layout())?;
    }

    let first_idx = page_idx(first_page, s.layout())?;

    let mut extra_photos: Vec<String> = Vec::new();
    for &p in other_pages {
        let idx = page_idx(p, s.layout())?;
        extra_photos.extend(s.layout()[idx].photos.clone());
    }

    s.layout_mut()[first_idx].photos.extend(extra_photos);
    s.layout_mut()[first_idx].slots.clear();

    let mut delete_indices: Vec<usize> = other_pages
        .iter()
        .map(|&p| page_idx(p, s.layout()).unwrap())
        .collect();
    delete_indices.sort_unstable_by(|a, b| b.cmp(a));
    for idx in &delete_indices {
        s.layout_mut().remove(*idx);
    }
    Ok(())
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
        assert_eq!(result.result.pages_deleted, vec![1, 2]);

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
