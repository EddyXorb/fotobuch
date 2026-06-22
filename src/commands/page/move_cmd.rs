//! `fotobuch page move` command.

use std::path::Path;

use crate::commands::{CommandOutput, run_write_command};
use crate::models::{LayoutPage, PageMode, Slot};
use crate::state_manager::{ReadOnlyState, WriteLayoutState};

use super::helpers::{
    apply_unplace, collect_src_photos, delete_empty_pages, format_pages_list, format_src_desc,
    page_idx, photos_at_slots, resolve_slots,
};
use super::swap::execute_swap;
use super::types::{
    DstMove, PageMoveCmd, PageMoveError, PageMoveResult, SlotExpr, Src, ValidationError,
};

/// Execute a `page move` command (either Move or Swap variant).
pub fn execute_move(
    project_root: &Path,
    cmd: PageMoveCmd,
) -> Result<CommandOutput<PageMoveResult>, PageMoveError> {
    match cmd {
        PageMoveCmd::Move { src, dst } => execute_move_to(project_root, src, dst),
        PageMoveCmd::Swap { left, right } => execute_swap(project_root, left, right),
    }
}

/// Cascade offset between consecutive slots when several photos are dropped onto
/// a Manual page in one gesture, so they don't perfectly overlap.
const MANUAL_DROP_CASCADE_MM: f64 = 10.0;

fn empty_result() -> PageMoveResult {
    PageMoveResult {
        pages_modified: vec![],
        pages_inserted: vec![],
        pages_deleted: vec![],
    }
}

fn execute_move_to(
    project_root: &Path,
    src: Src,
    dst: DstMove,
) -> Result<CommandOutput<PageMoveResult>, PageMoveError> {
    if let DstMove::ManualAt { page, x_mm, y_mm } = dst {
        return execute_move_to_manual(project_root, src, page, x_mm, y_mm);
    }

    // Same-page slot move is a no-op — don't even open the project.
    if let Src::Slots { page, slots: _ } = &src
        && let DstMove::Page(dst_page) = &dst
        && *page == *dst_page
    {
        return Ok(CommandOutput {
            result: empty_result(),
            changed_state: None,
        });
    }

    run_write_command(project_root, |mgr| {
        let mut view = mgr.get_write_layout_state();
        apply_move(&mut view, src, dst)
    })
}

/// Orchestrate a (non-manual, non-same-page) move on the layout view.
fn apply_move(
    s: &mut WriteLayoutState,
    src: Src,
    dst: DstMove,
) -> Result<(String, PageMoveResult), PageMoveError> {
    if matches!(dst, DstMove::Unplace) {
        return move_to_unplace(s, src);
    }

    let photos = collect_src_photos(&src, s.layout())?.0;
    if photos.is_empty() {
        return Ok((String::new(), empty_result()));
    }

    // For Slots: resolve src index and slot indices BEFORE any insertion so that
    // a NewPageAt insert cannot shift the src page out of position.
    let pre_insert_src = if let Src::Slots { page, slots } = &src {
        let idx = page_idx(*page, s.layout())?;
        let slot_indices = resolve_slots(*page, slots, s.layout())?;
        Some((idx, slot_indices))
    } else {
        None
    };

    let (dst_page_idx, inserted_page) = resolve_move_dst(s, &dst)?;

    if let Src::Slots { page, .. } = &src {
        let pre_insert = pre_insert_src.expect("Slots arm has pre_insert_src");
        return Ok(move_slots_to_page(
            s,
            *page,
            pre_insert,
            &photos,
            dst_page_idx,
            inserted_page,
        ));
    }

    move_pages_to_page(s, &src, &photos, dst_page_idx, inserted_page)
}

/// Remove photos from the layout (unplace destination).
fn move_to_unplace(
    s: &mut WriteLayoutState,
    src: Src,
) -> Result<(String, PageMoveResult), PageMoveError> {
    match src {
        Src::Slots { page, slots } => {
            let (deleted, modified) = apply_unplace(s.layout_mut(), page, &slots)?;
            Ok((
                format!("page move: page {page}:... -> (unplace)"),
                PageMoveResult {
                    pages_modified: modified,
                    pages_inserted: vec![],
                    pages_deleted: deleted,
                },
            ))
        }
        Src::Pages(pe) => {
            // Remove pages descending so indices stay valid.
            let mut page_nums = pe.pages.clone();
            if s.state().has_cover() && page_nums.contains(&0) {
                return Err(ValidationError::PageNotFound(0).into());
            }
            let src_desc = format_pages_list(&pe.pages);
            page_nums.sort_unstable_by(|a, b| b.cmp(a));
            let mut deleted = vec![];
            for &p in &page_nums {
                let idx = page_idx(p, s.layout())?;
                deleted.push(idx as u32);
                s.layout_mut().remove(idx);
            }
            deleted.sort();
            Ok((
                format!("page move: {src_desc} -> (unplace)"),
                PageMoveResult {
                    pages_modified: vec![],
                    pages_inserted: vec![],
                    pages_deleted: deleted,
                },
            ))
        }
    }
}

/// Resolve the destination page index, inserting a new page for `NewPageAt`.
/// Returns `(dst_page_idx, inserted_page_num)`.
fn resolve_move_dst(
    s: &mut WriteLayoutState,
    dst: &DstMove,
) -> Result<(usize, Option<u32>), PageMoveError> {
    match dst {
        DstMove::Page(p) => Ok((page_idx(*p, s.layout())?, None)),
        DstMove::NewPageAt(idx) => {
            if (*idx as usize) > s.layout().len() {
                return Err(ValidationError::PageNotFound(*idx).into());
            }
            if *idx == 0 && s.state().has_cover() {
                return Err(ValidationError::PageNotFound(0).into());
            }
            let new_idx = *idx as usize;
            s.layout_mut().insert(
                new_idx,
                LayoutPage {
                    photos: vec![],
                    slots: vec![],
                    mode: PageMode::Auto,
                },
            );
            Ok((new_idx, Some(new_idx as u32)))
        }
        DstMove::Unplace => unreachable!("Unplace handled above"),
        DstMove::ManualAt { .. } => unreachable!("ManualAt handled above"),
    }
}

/// Move individual slots from their source page onto the destination page.
fn move_slots_to_page(
    s: &mut WriteLayoutState,
    src_page: u32,
    pre_insert: (usize, Vec<usize>),
    photos: &[String],
    dst_page_idx: usize,
    inserted_page: Option<u32>,
) -> (String, PageMoveResult) {
    let (pre_idx, slot_indices) = pre_insert;
    let idx = if inserted_page.is_some() && dst_page_idx <= pre_idx {
        pre_idx + 1
    } else {
        pre_idx
    };
    let dst_page_num = dst_page_idx as u32;
    let mut desc = slot_indices.clone();
    desc.sort_unstable_by(|a, b| b.cmp(a));

    let src_is_manual = s.layout()[idx].mode == PageMode::Manual;
    for &i in &desc {
        s.layout_mut()[idx].photos.remove(i);
        if src_is_manual && i < s.layout()[idx].slots.len() {
            s.layout_mut()[idx].slots.remove(i);
        }
    }
    for photo in photos {
        s.layout_mut()[dst_page_idx].photos.push(photo.clone());
    }
    let deleted = delete_empty_pages(s.layout_mut());
    let mut modified = vec![src_page, dst_page_num];
    modified.retain(|p| !deleted.contains(p));
    modified.sort();
    modified.dedup();

    (
        format!("page move: slots from page {src_page} -> page"),
        PageMoveResult {
            pages_modified: modified,
            pages_inserted: inserted_page
                .map(|_| vec![dst_page_num])
                .unwrap_or_default(),
            pages_deleted: deleted,
        },
    )
}

/// Move whole source pages' photos onto the destination page.
fn move_pages_to_page(
    s: &mut WriteLayoutState,
    src: &Src,
    photos: &[String],
    dst_page_idx: usize,
    inserted_page: Option<u32>,
) -> Result<(String, PageMoveResult), PageMoveError> {
    let src_page_indices: Vec<usize> = match src {
        Src::Pages(pe) => pe
            .pages
            .iter()
            .map(|&p| page_idx(p, s.layout()))
            .collect::<Result<Vec<_>, _>>()?,
        _ => unreachable!(),
    };
    for &idx in &src_page_indices {
        s.layout_mut()[idx].photos.clear();
    }
    for photo in photos {
        s.layout_mut()[dst_page_idx].photos.push(photo.clone());
    }
    let dst_page_num = dst_page_idx as u32;
    let deleted = delete_empty_pages(s.layout_mut());
    let mut modified = vec![dst_page_num];
    modified.retain(|p| !deleted.contains(p));
    let src_desc = format_src_desc(src);

    Ok((
        format!("page move: {src_desc} -> page {dst_page_num}"),
        PageMoveResult {
            pages_modified: modified,
            pages_inserted: inserted_page
                .map(|_| vec![dst_page_num])
                .unwrap_or_default(),
            pages_deleted: deleted,
        },
    ))
}

/// Move the source slots onto a Manual-mode page, creating a positioned slot for
/// each moved photo. New slots keep the size of their source slot; the first is
/// placed with its top-left at `(x_mm, y_mm)`, further ones cascade.
fn execute_move_to_manual(
    project_root: &Path,
    src: Src,
    dst_page: u32,
    x_mm: f64,
    y_mm: f64,
) -> Result<CommandOutput<PageMoveResult>, PageMoveError> {
    let Src::Slots {
        page: src_page,
        slots,
    } = src
    else {
        // Whole-page moves onto a manual page are not supported via this path.
        return Err(ValidationError::PageNotManual(dst_page).into());
    };

    run_write_command(project_root, |mgr| {
        let mut view = mgr.get_write_layout_state();
        apply_move_to_manual(&mut view, src_page, &slots, dst_page, x_mm, y_mm)
    })
}

/// Mutating core of `execute_move_to_manual`, working on the layout view.
fn apply_move_to_manual(
    s: &mut WriteLayoutState,
    src_page: u32,
    slots: &SlotExpr,
    dst_page: u32,
    x_mm: f64,
    y_mm: f64,
) -> Result<(String, PageMoveResult), PageMoveError> {
    let dst_idx = page_idx(dst_page, s.layout())?;
    if s.layout()[dst_idx].mode != PageMode::Manual {
        return Err(ValidationError::PageNotManual(dst_page).into());
    }
    if src_page == dst_page {
        // Same page → a free reposition, handled by `page pos`, not here.
        return Ok((String::new(), empty_result()));
    }

    let src_idx = page_idx(src_page, s.layout())?;
    let mut slot_indices = resolve_slots(src_page, slots, s.layout())?;
    slot_indices.sort_unstable();

    // Snapshot each moved photo together with the size of its source slot.
    let mut moved: Vec<(String, f64, f64)> = Vec::with_capacity(slot_indices.len());
    for &i in &slot_indices {
        let photo = photos_at_slots(s.layout(), src_idx, &[i])?.remove(0);
        let slot_dims = s.layout()[src_idx]
            .slots
            .get(i)
            .map(|sl| (sl.width_mm, sl.height_mm));
        let (w, h) = match slot_dims {
            Some((w, h)) => (w, h),
            None => default_manual_slot_size(s.state(), dst_idx),
        };
        moved.push((photo, w, h));
    }

    // Remove from src (descending so indices stay valid). Drop the slot only on a
    // manual source; Auto pages recompute their slots on the next build.
    let src_is_manual = s.layout()[src_idx].mode == PageMode::Manual;
    let mut desc = slot_indices.clone();
    desc.sort_unstable_by(|a, b| b.cmp(a));
    for &i in &desc {
        s.layout_mut()[src_idx].photos.remove(i);
        if src_is_manual && i < s.layout()[src_idx].slots.len() {
            s.layout_mut()[src_idx].slots.remove(i);
        }
    }

    // Append photos and matching positioned slots to the manual destination.
    for (n, (photo, w, h)) in moved.into_iter().enumerate() {
        let offset = MANUAL_DROP_CASCADE_MM * n as f64;
        s.layout_mut()[dst_idx].photos.push(photo);
        s.layout_mut()[dst_idx].slots.push(Slot {
            x_mm: x_mm + offset,
            y_mm: y_mm + offset,
            width_mm: w,
            height_mm: h,
        });
    }

    let dst_page_num = dst_idx as u32;
    let deleted = delete_empty_pages(s.layout_mut());
    let mut modified = vec![src_page, dst_page_num];
    modified.retain(|p| !deleted.contains(p));
    modified.sort_unstable();
    modified.dedup();

    Ok((
        format!("page move: slots from page {src_page} -> manual page {dst_page_num}"),
        PageMoveResult {
            pages_modified: modified,
            pages_inserted: vec![],
            pages_deleted: deleted,
        },
    ))
}

/// Fallback slot size when a source slot has no computed geometry yet
/// (e.g. an Auto page that was never built): 30 % of the destination page.
fn default_manual_slot_size(state: &crate::models::ProjectState, dst_idx: usize) -> (f64, f64) {
    let (pw, ph) = state.page_dimensions_mm(dst_idx);
    (pw * 0.3, ph * 0.3)
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
        assert!(result.result.pages_deleted.contains(&1));

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout.len(), 1);
        let page1 = &mgr.state().layout[0];
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
        assert_eq!(result.result.pages_deleted, vec![0]);
        assert!(result.result.pages_modified.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout.len(), 1);
        assert_eq!(mgr.state().layout[0].photos, vec!["p2.jpg"]);
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
        assert_eq!(result.result.pages_modified, vec![0]);
        assert!(result.result.pages_deleted.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout[0].photos, vec!["p2.jpg"]);
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
            dst: DstMove::NewPageAt(1),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(!result.result.pages_inserted.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout.len(), 3);
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
            dst: DstMove::NewPageAt(1),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(!result.result.pages_inserted.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        // Original page 0, new page (with p1.jpg), original page 1 (with p2.jpg)
        assert_eq!(mgr.state().layout.len(), 3);
        assert_eq!(mgr.state().layout[0].photos, vec!["p0.jpg"]);
        assert!(mgr.state().layout[1].photos.contains(&"p1.jpg".to_owned()));
        assert!(mgr.state().layout[2].photos.contains(&"p2.jpg".to_owned()));
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
        assert!(result.result.pages_deleted.contains(&0));

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout.len(), 1);
        assert!(mgr.state().layout[0].photos.contains(&"p0.jpg".to_owned()));
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
        assert!(result.result.pages_deleted.contains(&0));
        assert!(result.result.pages_modified.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout.len(), 1);
        assert_eq!(mgr.state().layout[0].photos, vec!["p2.jpg"]);
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
        assert_eq!(result.result.pages_modified, vec![0, 1, 2, 3]);

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(
            mgr.state().layout[0].photos,
            vec!["c1.jpg", "c2.jpg", "c3.jpg"]
        );
        assert_eq!(mgr.state().layout[1].photos, vec!["d1.jpg"]);
        assert_eq!(mgr.state().layout[2].photos, vec!["a1.jpg", "a2.jpg"]);
        assert_eq!(mgr.state().layout[3].photos, vec!["b1.jpg"]);
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
        assert_eq!(result.result.pages_modified, vec![0, 1, 3, 4, 5]);

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout[0].photos, vec!["c.jpg"]);
        assert_eq!(mgr.state().layout[1].photos, vec!["d.jpg"]);
        assert_eq!(mgr.state().layout[2].photos, vec!["e.jpg"]);
        assert_eq!(mgr.state().layout[3].photos, vec!["M.jpg"]);
        assert_eq!(mgr.state().layout[4].photos, vec!["a.jpg"]);
        assert_eq!(mgr.state().layout[5].photos, vec!["b.jpg"]);
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
        assert_eq!(
            mgr.state().layout[0].photos,
            vec!["c.jpg", "b.jpg", "a.jpg"]
        );
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

    #[test]
    fn execute_move_new_page_at_zero_inserts_before_first_page() {
        // Page 1 has two photos so it won't be deleted after the move.
        let state = make_state_with_layout(vec![vec!["a.jpg"], vec!["b.jpg", "c.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 1,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::NewPageAt(0),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(!result.result.pages_inserted.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        // new page(b.jpg), page0(a.jpg), page1(c.jpg)
        assert_eq!(mgr.state().layout.len(), 3);
        assert!(mgr.state().layout[0].photos.contains(&"b.jpg".to_owned()));
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn execute_move_new_page_at_len_appends() {
        let state = make_state_with_layout(vec![vec!["a.jpg", "b.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::NewPageAt(1),
        };
        execute_move(tmp.path(), cmd).unwrap();

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout.len(), 2);
        assert!(mgr.state().layout[1].photos.contains(&"a.jpg".to_owned()));
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn move_slot_into_manual_page_creates_positioned_slot() {
        use super::super::mode::execute_mode;
        use crate::models::PageMode;

        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"], vec!["p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);
        // Page 1 becomes Manual.
        execute_mode(tmp.path(), PagesExpr::single(1), PageMode::Manual).unwrap();

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::ManualAt {
                page: 1,
                x_mm: 50.0,
                y_mm: 60.0,
            },
        };
        execute_move(tmp.path(), cmd).unwrap();

        let mgr = StateManager::open(tmp.path()).unwrap();
        let dst = &mgr.state().layout[1];
        // Manual page now holds both photos with matching slot counts.
        assert_eq!(dst.photos, vec!["p2.jpg", "p0.jpg"]);
        assert_eq!(dst.slots.len(), dst.photos.len());
        // New slot keeps the source slot size (fixture: 100 x 80) at the drop point.
        let new = dst.slots.last().unwrap();
        assert_eq!((new.x_mm, new.y_mm), (50.0, 60.0));
        assert_eq!((new.width_mm, new.height_mm), (100.0, 80.0));
        // Source page lost the moved photo.
        assert_eq!(mgr.state().layout[0].photos, vec!["p1.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn move_into_non_manual_page_is_rejected() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"], vec!["p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::ManualAt {
                page: 1,
                x_mm: 0.0,
                y_mm: 0.0,
            },
        };
        let err = execute_move(tmp.path(), cmd).unwrap_err();
        assert!(matches!(
            err,
            PageMoveError::Validation(ValidationError::PageNotManual(1))
        ));
    }

    #[test]
    fn swap_into_manual_page_adapts_slot_height_to_photo_ratio() {
        use super::super::mode::execute_mode;
        use super::super::types::DstSwap;
        use crate::models::PageMode;

        // Fixture photos are 4000 x 3000 (4:3).
        let state = make_state_with_layout(vec![vec!["p0.jpg"], vec!["p1.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);
        execute_mode(tmp.path(), PagesExpr::single(1), PageMode::Manual).unwrap();

        let cmd = PageMoveCmd::Swap {
            left: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            right: DstSwap::Slots {
                page: 1,
                slots: SlotExpr::single(0),
            },
        };
        execute_move(tmp.path(), cmd).unwrap();

        let mgr = StateManager::open(tmp.path()).unwrap();
        let manual_slot = &mgr.state().layout[1].slots[0];
        // Width kept (100), height now matches the incoming photo's ratio: 100 * 3/4 = 75.
        assert_eq!(manual_slot.width_mm, 100.0);
        assert!((manual_slot.height_mm - 75.0).abs() < 1e-9);
        assert_eq!(mgr.state().layout[1].photos, vec!["p0.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn execute_move_new_page_at_out_of_range_errors() {
        let state = make_state_with_layout(vec![vec!["a.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::NewPageAt(99),
        };
        let err = execute_move(tmp.path(), cmd).unwrap_err();
        assert!(matches!(
            err,
            PageMoveError::Validation(ValidationError::PageNotFound(99))
        ));
    }

    #[test]
    fn execute_move_pages_unplace_rejects_cover_when_active() {
        use crate::models::{BookConfig, CoverConfig, ProjectConfig};
        let mut state = make_state_with_layout(vec![vec!["cover.jpg"], vec!["a.jpg"]]);
        state.config = ProjectConfig {
            book: BookConfig {
                cover: CoverConfig {
                    active: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Pages(PagesExpr::single(0)),
            dst: DstMove::Unplace,
        };
        let err = execute_move(tmp.path(), cmd).unwrap_err();
        assert!(matches!(
            err,
            PageMoveError::Validation(ValidationError::PageNotFound(0))
        ));
    }
}
