//! `fotobuch page move` and `fotobuch page swap` commands.

use std::path::Path;

use crate::dto_models::LayoutPage;
use crate::state_manager::StateManager;

use super::helpers::{
    collect_dst_swap_photos_with_indices, collect_src_photos, collect_src_photos_with_indices,
    delete_empty_pages, format_pages_list, format_src_desc, page_idx, remove_slots, resolve_slots,
};
use super::types::{
    DstMove, DstSwap, PageMoveCmd, PageMoveError, PageMoveResult, Src, ValidationError,
};

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

    // Handle unplace-destination: remove photos from layout (and delete pages for Src::Pages).
    if matches!(dst, DstMove::Unplace) {
        return match src {
            Src::Slots { page, slots } => {
                let idx = page_idx(page, &mgr.state.layout)?;
                let slot_indices = resolve_slots(page, &slots, &mgr.state.layout)?;
                remove_slots(&mut mgr.state.layout, idx, slot_indices);
                let deleted = delete_empty_pages(&mut mgr.state.layout);
                let modified = if deleted.contains(&page) {
                    vec![]
                } else {
                    vec![page]
                };
                mgr.finish(&format!("page move: page {page}:... -> (unplace)"))?;
                Ok(PageMoveResult {
                    pages_modified: modified,
                    pages_inserted: vec![],
                    pages_deleted: deleted,
                })
            }
            Src::Pages(pe) => {
                // Remove pages descending so indices stay valid.
                let mut page_nums = pe.pages.clone();
                let src_desc = format_pages_list(&pe.pages);
                page_nums.sort_unstable_by(|a, b| b.cmp(a));
                let mut deleted = vec![];
                for &p in &page_nums {
                    let idx = page_idx(p, &mgr.state.layout)?;
                    let page_num = mgr.state.layout[idx].page as u32;
                    mgr.state.layout.remove(idx);
                    deleted.push(page_num);
                }
                deleted.sort();
                mgr.finish(&format!("page move: {src_desc} -> (unplace)"))?;
                Ok(PageMoveResult {
                    pages_modified: vec![],
                    pages_inserted: vec![],
                    pages_deleted: deleted,
                })
            }
        };
    }

    let (photos, _src_page_indices) = collect_src_photos(&src, &mgr.state.layout)?;
    if photos.is_empty() {
        return Ok(PageMoveResult {
            pages_modified: vec![],
            pages_inserted: vec![],
            pages_deleted: vec![],
        });
    }

    // For Slots: resolve src index and slot indices BEFORE any insertion so that
    // a NewPageAfter insert cannot shift the src page out of position.
    let pre_insert_src = if let Src::Slots { page, slots } = &src {
        let idx = page_idx(*page, &mgr.state.layout)?;
        let slot_indices = resolve_slots(*page, slots, &mgr.state.layout)?;
        Some((idx, slot_indices))
    } else {
        None
    };

    let (dst_page_idx, inserted_page) = match &dst {
        DstMove::Page(p) => {
            let idx = page_idx(*p, &mgr.state.layout)?;
            (idx, None)
        }
        DstMove::NewPageAfter(p) => {
            let after_idx = page_idx(*p, &mgr.state.layout)?;
            let new_idx = after_idx + 1;
            let new_page_num = new_idx; // will be renumbered by finish()
            mgr.state.layout.insert(
                new_idx,
                LayoutPage {
                    page: new_page_num,
                    photos: vec![],
                    slots: vec![],

                    mode: None,
                },
            );
            (new_idx, Some(new_page_num as u32))
        }
        DstMove::Unplace => unreachable!("Unplace handled above"),
    };

    // For Slots variant: remove photos from src and add to dst.
    if let Src::Slots { page, .. } = &src {
        let src_page = *page;
        // Adjust src index if the new-page insert shifted it.
        let (idx, slot_indices) = {
            let (pre_idx, slot_indices) = pre_insert_src.expect("Slots arm has pre_insert_src");
            let idx = if inserted_page.is_some() && dst_page_idx <= pre_idx {
                pre_idx + 1
            } else {
                pre_idx
            };
            (idx, slot_indices)
        };
        let dst_page_num = mgr.state.layout[dst_page_idx].page as u32;

        // Remove photos from src slots (descending to keep indices stable)
        let mut desc = slot_indices.clone();
        desc.sort_unstable_by(|a, b| b.cmp(a));
        for &i in &desc {
            mgr.state.layout[idx].photos.remove(i);
        }

        // Add photos to dst
        for photo in &photos {
            mgr.state.layout[dst_page_idx].photos.push(photo.clone());
        }

        let deleted = delete_empty_pages(&mut mgr.state.layout);
        let mut modified = vec![src_page, dst_page_num];
        modified.retain(|p| !deleted.contains(p));
        modified.sort();
        modified.dedup();
        mgr.finish(&format!("page move: slots from page {src_page} -> page"))?;
        return Ok(PageMoveResult {
            pages_modified: modified,
            pages_inserted: inserted_page
                .map(|_| vec![dst_page_num])
                .unwrap_or_default(),
            pages_deleted: deleted,
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

    for &idx in &src_page_indices {
        mgr.state.layout[idx].photos.clear();
    }

    for photo in &photos {
        mgr.state.layout[dst_page_idx].photos.push(photo.clone());
    }
    let dst_page_num = mgr.state.layout[dst_page_idx].page as u32;

    let deleted = delete_empty_pages(&mut mgr.state.layout);
    let mut modified_pages = vec![dst_page_num];
    modified_pages.retain(|p| !deleted.contains(p));

    let src_desc = format_src_desc(&src);
    mgr.finish(&format!("page move: {src_desc} -> page {dst_page_num}"))?;

    Ok(PageMoveResult {
        pages_modified: modified_pages,
        pages_inserted: inserted_page
            .map(|_| vec![dst_page_num])
            .unwrap_or_default(),
        pages_deleted: deleted,
    })
}

fn execute_swap(
    project_root: &Path,
    left: Src,
    right: DstSwap,
) -> Result<PageMoveResult, PageMoveError> {
    let mut mgr = StateManager::open(project_root)?;

    // Pages × Pages — block transposition, contiguous ranges only.
    if let (Src::Pages(lpe), DstSwap::Pages(rpe)) = (&left, &right) {
        if !is_contiguous(&lpe.pages) || !is_contiguous(&rpe.pages) {
            return Err(ValidationError::SwapNonContiguous.into());
        }

        let left_set: std::collections::HashSet<u32> = lpe.pages.iter().copied().collect();
        if rpe.pages.iter().any(|p| left_set.contains(p)) {
            return Err(ValidationError::SwapRangesOverlap.into());
        }

        for &p in lpe.pages.iter().chain(rpe.pages.iter()) {
            page_idx(p, &mgr.state.layout)?;
        }

        let mut modified_pages: Vec<u32> =
            lpe.pages.iter().chain(rpe.pages.iter()).copied().collect();
        modified_pages.sort();
        modified_pages.dedup();

        block_transpose_pages(&mut mgr.state.layout, &lpe.pages, &rpe.pages);

        mgr.finish("page swap")?;
        return Ok(PageMoveResult {
            pages_modified: modified_pages,
            pages_inserted: vec![],
            pages_deleted: vec![],
        });
    }

    // Slot swap — blockwise replacement, contiguous ranges only.
    if !src_is_contiguous(&left) || !dst_swap_is_contiguous(&right) {
        return Err(ValidationError::SwapNonContiguous.into());
    }

    let (left_photos, left_page_idx, left_slot_indices) =
        collect_src_photos_with_indices(&left, &mgr.state.layout)?;
    let (right_photos, right_page_idx, right_slot_indices) =
        collect_dst_swap_photos_with_indices(&right, &mgr.state.layout)?;

    // Same page: slot ranges must not overlap.
    if left_page_idx == right_page_idx {
        let left_set: std::collections::HashSet<usize> =
            left_slot_indices.iter().copied().collect();
        if right_slot_indices.iter().any(|i| left_set.contains(i)) {
            return Err(ValidationError::SwapRangesOverlap.into());
        }
    }

    swap_photos_in_layout(
        &mut mgr.state.layout,
        SwapSide {
            page_idx: left_page_idx,
            slot_indices: &left_slot_indices,
            photos: &left_photos,
        },
        SwapSide {
            page_idx: right_page_idx,
            slot_indices: &right_slot_indices,
            photos: &right_photos,
        },
    );

    let mut modified_pages = vec![
        mgr.state.layout[left_page_idx].page as u32,
        mgr.state.layout[right_page_idx].page as u32,
    ];
    modified_pages.sort();
    modified_pages.dedup();

    mgr.finish("page swap")?;

    Ok(PageMoveResult {
        pages_modified: modified_pages,
        pages_inserted: vec![],
        pages_deleted: vec![],
    })
}

fn is_contiguous(items: &[u32]) -> bool {
    items.len() <= 1 || items.windows(2).all(|w| w[1] == w[0] + 1)
}

fn src_is_contiguous(src: &Src) -> bool {
    match src {
        Src::Pages(pe) => is_contiguous(&pe.pages),
        Src::Slots { slots, .. } => slots.items.len() <= 1,
    }
}

fn dst_swap_is_contiguous(dst: &DstSwap) -> bool {
    match dst {
        DstSwap::Pages(pe) => is_contiguous(&pe.pages),
        DstSwap::Slots { slots, .. } => slots.items.len() <= 1,
    }
}

/// Block-transpose two contiguous page ranges within the layout.
/// `left_pages` and `right_pages` are 0-based (matching `LayoutPage.page`), contiguous, non-overlapping.
/// The block that starts first (by page number) is treated as the "left" block.
fn block_transpose_pages(layout: &mut Vec<LayoutPage>, left_pages: &[u32], right_pages: &[u32]) {
    let l0 = page_idx(left_pages[0], layout).unwrap();
    let l1 = page_idx(*left_pages.last().unwrap(), layout).unwrap();
    let r0 = page_idx(right_pages[0], layout).unwrap();
    let r1 = page_idx(*right_pages.last().unwrap(), layout).unwrap();

    // Normalize so that (l0..=l1) comes before (r0..=r1).
    let (l0, l1, r0, r1) = if l0 <= r0 {
        (l0, l1, r0, r1)
    } else {
        (r0, r1, l0, l1)
    };

    // Drain the full segment [l0..=r1] and reassemble as right + middle + left.
    let segment: Vec<LayoutPage> = layout.drain(l0..=r1).collect();
    let left_len = l1 - l0 + 1;
    let right_start = r0 - l0;
    let right_len = r1 - r0 + 1;

    let mut new_segment = Vec::with_capacity(segment.len());
    new_segment.extend_from_slice(&segment[right_start..right_start + right_len]);
    new_segment.extend_from_slice(&segment[left_len..right_start]);
    new_segment.extend_from_slice(&segment[..left_len]);

    for (i, page) in new_segment.into_iter().enumerate() {
        layout.insert(l0 + i, page);
    }
}

struct SwapSide<'a> {
    page_idx: usize,
    slot_indices: &'a [usize],
    photos: &'a [String],
}

fn swap_photos_in_layout(layout: &mut [LayoutPage], left: SwapSide, right: SwapSide) {
    let swap_slots = left.photos.len() != right.photos.len();

    // Remove left photos (descending order to keep indices stable)
    let mut left_desc: Vec<usize> = left.slot_indices.to_vec();
    left_desc.sort_unstable_by(|a, b| b.cmp(a));
    for &i in &left_desc {
        layout[left.page_idx].photos.remove(i);
        if swap_slots && i < layout[left.page_idx].slots.len() {
            layout[left.page_idx].slots.remove(i);
        }
    }

    let insert_at = left.slot_indices.iter().min().copied().unwrap_or(0);
    for (j, photo) in right.photos.iter().enumerate() {
        let pos = (insert_at + j).min(layout[left.page_idx].photos.len());
        layout[left.page_idx].photos.insert(pos, photo.clone());
    }

    let mut right_desc: Vec<usize> = right.slot_indices.to_vec();
    right_desc.sort_unstable_by(|a, b| b.cmp(a));
    for &i in &right_desc {
        layout[right.page_idx].photos.remove(i);
        if swap_slots && i < layout[right.page_idx].slots.len() {
            layout[right.page_idx].slots.remove(i);
        }
    }

    let insert_at_r = right.slot_indices.iter().min().copied().unwrap_or(0);
    for (j, photo) in left.photos.iter().enumerate() {
        let pos = (insert_at_r + j).min(layout[right.page_idx].photos.len());
        layout[right.page_idx].photos.insert(pos, photo.clone());
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::{make_state_with_layout, setup_repo};
    use super::super::types::{DstMove, PageMoveCmd, PagesExpr, SlotExpr, Src};
    use super::*;
    use crate::state_manager::StateManager;
    use tempfile::TempDir;

    #[test]
    fn test_execute_move_pages_to_page() {
        let state =
            make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"], vec!["p2.jpg", "p3.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Pages(PagesExpr::single(1)),
            dst: DstMove::Page(0),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(result.pages_deleted.contains(&1));

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state.layout.len(), 1);
        let page1 = &mgr.state.layout[0];
        assert!(page1.photos.contains(&"p2.jpg".to_owned()));
        assert!(page1.photos.contains(&"p3.jpg".to_owned()));
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_move_unplace_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"], vec!["p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Pages(PagesExpr::single(0)),
            dst: DstMove::Unplace,
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert_eq!(result.pages_deleted, vec![0]);
        assert!(result.pages_modified.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state.layout.len(), 1);
        assert_eq!(mgr.state.layout[0].photos, vec!["p2.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_move_unplace_slots() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg", "p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::from_range(0, 1),
            },
            dst: DstMove::Unplace,
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert_eq!(result.pages_modified, vec![0]);
        assert!(result.pages_deleted.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state.layout[0].photos, vec!["p2.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_move_to_new_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"], vec!["p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::NewPageAfter(0),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(!result.pages_inserted.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state.layout.len(), 3);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_move_slots_to_new_page_after() {
        // Regression: "page move 2:1 to 1+" must not fail with SlotNotFound.
        // Inserting the new page after page 1 shifts page 2 from index 1 to index 2;
        // slot resolution must happen before the insert.
        let state = make_state_with_layout(vec![vec!["p0.jpg"], vec!["p1.jpg", "p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 1,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::NewPageAfter(0),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(!result.pages_inserted.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        // Original page 0, new page (with p1.jpg), original page 1 (with p2.jpg)
        assert_eq!(mgr.state.layout.len(), 3);
        assert_eq!(mgr.state.layout[0].photos, vec!["p0.jpg"]);
        assert!(mgr.state.layout[1].photos.contains(&"p1.jpg".to_owned()));
        assert!(mgr.state.layout[2].photos.contains(&"p2.jpg".to_owned()));
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_move_slots_to_page_deletes_empty_src() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"], vec!["p1.jpg", "p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        // Move the only slot from page 0 to page 1 → page 0 becomes empty → deleted
        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::Page(1),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(result.pages_deleted.contains(&0));

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state.layout.len(), 1);
        assert!(mgr.state.layout[0].photos.contains(&"p0.jpg".to_owned()));
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_move_unplace_all_slots_deletes_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"], vec!["p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        // Unplace all slots from page 0 → page 0 becomes empty → deleted
        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::from_range(0, 1),
            },
            dst: DstMove::Unplace,
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(result.pages_deleted.contains(&0));
        assert!(result.pages_modified.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state.layout.len(), 1);
        assert_eq!(mgr.state.layout[0].photos, vec!["p2.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_swap_page_range_block_transposition() {
        let state = make_state_with_layout(vec![
            vec!["a1.jpg", "a2.jpg"],           // page 1
            vec!["b1.jpg"],                     // page 2
            vec!["c1.jpg", "c2.jpg", "c3.jpg"], // page 3
            vec!["d1.jpg"],                     // page 4
        ]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        // Equal-size block transposition: [0,1] ↔ [2,3]
        let cmd = PageMoveCmd::Swap {
            left: Src::Pages(PagesExpr::from_range(0, 1)),
            right: super::super::types::DstSwap::Pages(PagesExpr::from_range(2, 3)),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert_eq!(result.pages_modified, vec![0, 1, 2, 3]);

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(
            mgr.state.layout[0].photos,
            vec!["c1.jpg", "c2.jpg", "c3.jpg"]
        );
        assert_eq!(mgr.state.layout[1].photos, vec!["d1.jpg"]);
        assert_eq!(mgr.state.layout[2].photos, vec!["a1.jpg", "a2.jpg"]);
        assert_eq!(mgr.state.layout[3].photos, vec!["b1.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_swap_page_range_unequal_sizes_with_middle() {
        // [0,1] ↔ [3,4,5] with page 2 as middle
        // before: [a, b, M, c, d, e]
        // after:  [c, d, e, M, a, b]
        let state = make_state_with_layout(vec![
            vec!["a.jpg"],
            vec!["b.jpg"],
            vec!["M.jpg"],
            vec!["c.jpg"],
            vec!["d.jpg"],
            vec!["e.jpg"],
        ]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Swap {
            left: Src::Pages(PagesExpr::from_range(0, 1)),
            right: super::super::types::DstSwap::Pages(PagesExpr::from_range(3, 5)),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert_eq!(result.pages_modified, vec![0, 1, 3, 4, 5]);

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state.layout[0].photos, vec!["c.jpg"]);
        assert_eq!(mgr.state.layout[1].photos, vec!["d.jpg"]);
        assert_eq!(mgr.state.layout[2].photos, vec!["e.jpg"]);
        assert_eq!(mgr.state.layout[3].photos, vec!["M.jpg"]);
        assert_eq!(mgr.state.layout[4].photos, vec!["a.jpg"]);
        assert_eq!(mgr.state.layout[5].photos, vec!["b.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_swap_page_range_non_contiguous_error() {
        let state = make_state_with_layout(vec![
            vec!["a.jpg"],
            vec!["b.jpg"],
            vec!["c.jpg"],
            vec!["d.jpg"],
        ]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Swap {
            left: Src::Pages(PagesExpr::from_list(vec![0, 2])),
            right: super::super::types::DstSwap::Pages(PagesExpr::from_list(vec![1, 3])),
        };
        let err = execute_move(tmp.path(), cmd).unwrap_err();
        assert!(matches!(
            err,
            PageMoveError::Validation(ValidationError::SwapNonContiguous)
        ));
    }

    #[test]
    fn test_execute_swap_page_range_overlap() {
        let state = make_state_with_layout(vec![vec!["a.jpg"], vec!["b.jpg"], vec!["c.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Swap {
            left: Src::Pages(PagesExpr::from_range(0, 1)),
            right: super::super::types::DstSwap::Pages(PagesExpr::from_range(1, 2)),
        };
        let err = execute_move(tmp.path(), cmd).unwrap_err();
        assert!(matches!(
            err,
            PageMoveError::Validation(ValidationError::SwapRangesOverlap)
        ));
    }

    #[test]
    fn test_execute_swap_same_page_slots_allowed() {
        // swap 0:0 0:2 — slot 0 and slot 2 on the same page swap positions.
        let state = make_state_with_layout(vec![vec!["a.jpg", "b.jpg", "c.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Swap {
            left: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            right: super::super::types::DstSwap::Slots {
                page: 0,
                slots: SlotExpr::single(2),
            },
        };
        execute_move(tmp.path(), cmd).unwrap();

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state.layout[0].photos, vec!["c.jpg", "b.jpg", "a.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_swap_same_page_slots_overlap_error() {
        let state = make_state_with_layout(vec![vec!["a.jpg", "b.jpg", "c.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Swap {
            left: Src::Slots {
                page: 0,
                slots: SlotExpr::from_range(0, 1),
            },
            right: super::super::types::DstSwap::Slots {
                page: 0,
                slots: SlotExpr::from_range(1, 2),
            },
        };
        let err = execute_move(tmp.path(), cmd).unwrap_err();
        assert!(matches!(
            err,
            PageMoveError::Validation(ValidationError::SwapRangesOverlap)
        ));
    }
}
