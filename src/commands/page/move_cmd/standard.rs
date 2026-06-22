//! Standard (non-manual) move operations on the layout.

use crate::commands::page::helpers::{
    apply_unplace, collect_src_photos, delete_empty_pages, format_pages_list, format_src_desc,
    page_idx, resolve_slots,
};
use crate::commands::page::types::{DstMove, PageMoveError, PageMoveResult, Src, ValidationError};
use crate::models::{LayoutPage, PageMode};
use crate::state_manager::{ReadOnlyState, WriteLayoutState};

/// Orchestrate a (non-manual, non-same-page) move on the layout view.
pub(super) fn apply_move(
    s: &mut WriteLayoutState,
    src: Src,
    dst: DstMove,
) -> Result<(String, PageMoveResult), PageMoveError> {
    if matches!(dst, DstMove::Unplace) {
        return move_to_unplace(s, src);
    }

    let photos = collect_src_photos(&src, s.layout())?.0;
    if photos.is_empty() {
        return Ok((String::new(), super::empty_result()));
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
