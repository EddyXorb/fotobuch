//! `fotobuch page move` and `fotobuch page swap` commands.

use std::path::Path;

use crate::dto_models::LayoutPage;
use crate::state_manager::StateManager;

use super::helpers::{
    collect_dst_swap_photos_with_indices, collect_src_photos, collect_src_photos_with_indices,
    format_src_desc, page_idx, remove_slots, resolve_slots, single_page_of_dst_swap,
    single_page_of_src,
};
use super::types::{DstMove, DstSwap, PageMoveCmd, PageMoveError, PageMoveResult, Src,
    ValidationError};

/// Execute a `page move` command (either Move or Swap variant).
pub fn execute_move(
    project_root: &Path,
    cmd: PageMoveCmd,
) -> Result<PageMoveResult, PageMoveError> {
    match cmd {
        PageMoveCmd::Move { src, dst } => execute_move_to(project_root, src, dst),
        PageMoveCmd::Swap { left, right } => execute_swap(project_root, left, right),
    }
}

fn execute_move_to(
    project_root: &Path,
    src: Src,
    dst: DstMove,
) -> Result<PageMoveResult, PageMoveError> {
    let mut mgr = StateManager::open(project_root)?;

    let (photos, _src_page_indices) = collect_src_photos(&src, &mgr.state.layout)?;
    if photos.is_empty() {
        return Ok(PageMoveResult {
            pages_modified: vec![],
            pages_inserted: vec![],
            pages_deleted: vec![],
        });
    }

    let (dst_page_idx, inserted_page) = match &dst {
        DstMove::Page(p) => {
            let idx = page_idx(*p, &mgr.state.layout)?;
            (idx, None)
        }
        DstMove::NewPageAfter(p) => {
            let after_idx = page_idx(*p, &mgr.state.layout)?;
            let new_idx = after_idx + 1;
            let new_page_num = new_idx + 1; // will be renumbered by finish()
            mgr.state.layout.insert(
                new_idx,
                LayoutPage {
                    page: new_page_num,
                    photos: vec![],
                    slots: vec![],
                },
            );
            (new_idx, Some(new_page_num as u32))
        }
    };

    // For Slots variant: remove the slots and add to dst, then return early.
    if let Src::Slots { page, slots } = &src {
        let src_page = *page;
        let idx = page_idx(src_page, &mgr.state.layout)?;
        let slot_indices = resolve_slots(src_page, slots, &mgr.state.layout)?;
        remove_slots(&mut mgr.state.layout, idx, slot_indices);
        for photo in &photos {
            mgr.state.layout[dst_page_idx].photos.push(photo.clone());
        }
        let mut modified = vec![src_page, dst_page_idx as u32 + 1];
        modified.sort();
        modified.dedup();
        mgr.finish(&format!("page move: slots from page {src_page} -> page"))?;
        return Ok(PageMoveResult {
            pages_modified: modified,
            pages_inserted: inserted_page
                .map(|_| vec![dst_page_idx as u32 + 1])
                .unwrap_or_default(),
            pages_deleted: vec![],
        });
    }

    // For Pages variant: recollect indices (page insert may have shifted them), clear, and add.
    let src_page_indices: Vec<usize> = match &src {
        Src::Pages(pe) => pe
            .pages
            .iter()
            .map(|&p| page_idx(p, &mgr.state.layout))
            .collect::<Result<Vec<_>, _>>()?,
        _ => unreachable!(),
    };

    let mut modified_pages: Vec<u32> = Vec::new();
    for &idx in &src_page_indices {
        let page_num = mgr.state.layout[idx].page as u32;
        mgr.state.layout[idx].photos.clear();
        mgr.state.layout[idx].slots.clear();
        modified_pages.push(page_num);
    }

    for photo in &photos {
        mgr.state.layout[dst_page_idx].photos.push(photo.clone());
    }
    let dst_page_num = mgr.state.layout[dst_page_idx].page as u32;
    modified_pages.push(dst_page_num);
    modified_pages.sort();
    modified_pages.dedup();

    let src_desc = format_src_desc(&src);
    mgr.finish(&format!("page move: {src_desc} -> page {dst_page_num}"))?;

    Ok(PageMoveResult {
        pages_modified: modified_pages,
        pages_inserted: inserted_page
            .map(|_| vec![dst_page_idx as u32 + 1])
            .unwrap_or_default(),
        pages_deleted: vec![],
    })
}

fn execute_swap(
    project_root: &Path,
    left: Src,
    right: DstSwap,
) -> Result<PageMoveResult, PageMoveError> {
    let mut mgr = StateManager::open(project_root)?;

    let (left_photos, left_page_idx, left_slot_indices) =
        collect_src_photos_with_indices(&left, &mgr.state.layout)?;
    let (right_photos, right_page_idx, right_slot_indices) =
        collect_dst_swap_photos_with_indices(&right, &mgr.state.layout)?;

    if let (Some(lp), Some(rp)) = (single_page_of_src(&left), single_page_of_dst_swap(&right)) {
        if lp == rp {
            return Err(ValidationError::SwapSamePage(lp).into());
        }
    }

    swap_photos_in_layout(
        &mut mgr.state.layout,
        left_page_idx,
        &left_slot_indices,
        &left_photos,
        right_page_idx,
        &right_slot_indices,
        &right_photos,
    );

    let mut modified_pages: Vec<u32> = Vec::new();
    modified_pages.push(mgr.state.layout[left_page_idx].page as u32);
    modified_pages.push(mgr.state.layout[right_page_idx].page as u32);
    modified_pages.sort();
    modified_pages.dedup();

    mgr.finish("page swap")?;

    Ok(PageMoveResult {
        pages_modified: modified_pages,
        pages_inserted: vec![],
        pages_deleted: vec![],
    })
}

fn swap_photos_in_layout(
    layout: &mut [LayoutPage],
    left_page_idx: usize,
    left_slot_indices: &[usize],
    left_photos: &[String],
    right_page_idx: usize,
    right_slot_indices: &[usize],
    right_photos: &[String],
) {
    // Remove left photos (descending order to keep indices stable)
    let mut left_desc: Vec<usize> = left_slot_indices.to_vec();
    left_desc.sort_unstable_by(|a, b| b.cmp(a));
    for &i in &left_desc {
        layout[left_page_idx].photos.remove(i);
        if i < layout[left_page_idx].slots.len() {
            layout[left_page_idx].slots.remove(i);
        }
    }

    let insert_at = left_slot_indices.iter().min().copied().unwrap_or(0);
    for (j, photo) in right_photos.iter().enumerate() {
        let pos = (insert_at + j).min(layout[left_page_idx].photos.len());
        layout[left_page_idx].photos.insert(pos, photo.clone());
    }

    let mut right_desc: Vec<usize> = right_slot_indices.to_vec();
    right_desc.sort_unstable_by(|a, b| b.cmp(a));
    for &i in &right_desc {
        layout[right_page_idx].photos.remove(i);
        if i < layout[right_page_idx].slots.len() {
            layout[right_page_idx].slots.remove(i);
        }
    }

    let insert_at_r = right_slot_indices.iter().min().copied().unwrap_or(0);
    for (j, photo) in left_photos.iter().enumerate() {
        let pos = (insert_at_r + j).min(layout[right_page_idx].photos.len());
        layout[right_page_idx].photos.insert(pos, photo.clone());
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_fixtures::{make_state_with_layout, setup_repo};
    use super::super::types::{DstMove, PageMoveCmd, PagesExpr, SlotExpr, Src};
    use crate::state_manager::StateManager;
    use tempfile::TempDir;

    #[test]
    fn test_execute_move_pages_to_page() {
        let state = make_state_with_layout(vec![
            vec!["p0.jpg", "p1.jpg"],
            vec!["p2.jpg", "p3.jpg"],
        ]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Pages(PagesExpr::single(2)),
            dst: DstMove::Page(1),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(result.pages_modified.contains(&1));

        let mgr = StateManager::open(tmp.path()).unwrap();
        let page1 = &mgr.state.layout[0];
        assert!(page1.photos.contains(&"p2.jpg".to_owned()));
        assert!(page1.photos.contains(&"p3.jpg".to_owned()));
        assert!(mgr.state.layout[1].photos.is_empty());
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_move_to_new_page() {
        let state = make_state_with_layout(vec![
            vec!["p0.jpg", "p1.jpg"],
            vec!["p2.jpg"],
        ]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 1,
                slots: SlotExpr::single(1),
            },
            dst: DstMove::NewPageAfter(1),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(!result.pages_inserted.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state.layout.len(), 3);
        mgr.finish("test: noop").unwrap();
    }
}
