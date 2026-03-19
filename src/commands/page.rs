//! `fotobuch page` and `fotobuch unplace` commands.
//!
//! # Architecture
//!
//! This module contains all types and execution logic for page manipulation.
//! The CLI layer (`src/cli/page.rs`) handles string parsing and calls into here.
//!
//! See the design document at `docs/design/cli/page.md`.

use std::path::Path;

use crate::dto_models::LayoutPage;
use crate::state_manager::StateManager;

// ── Address types ─────────────────────────────────────────────────────────────

/// A list of page numbers: `3`, `3,5`, or `3..5`.
#[derive(Debug, Clone, PartialEq)]
pub struct PagesExpr {
    pub pages: Vec<u32>,
}

impl PagesExpr {
    pub fn single(page: u32) -> Self {
        Self { pages: vec![page] }
    }

    pub fn from_list(pages: Vec<u32>) -> Self {
        Self { pages }
    }

    pub fn from_range(start: u32, end: u32) -> Self {
        Self {
            pages: (start..=end).collect(),
        }
    }
}

/// A set of slot indices: `2`, `2,7`, `2..5`, or `2..5,7`.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotExpr {
    pub slots: Vec<u32>,
}

impl SlotExpr {
    pub fn single(slot: u32) -> Self {
        Self { slots: vec![slot] }
    }

    pub fn from_list(slots: Vec<u32>) -> Self {
        Self { slots }
    }

    pub fn from_range(start: u32, end: u32) -> Self {
        Self {
            slots: (start..=end).collect(),
        }
    }
}

/// Source address for `page move` and `page swap`.
#[derive(Debug, Clone, PartialEq)]
pub enum Src {
    /// One or more full pages (all photos on those pages).
    Pages(PagesExpr),
    /// Specific slots on a single page.
    Slots { page: u32, slots: SlotExpr },
}

/// Destination for `page move ->`.
#[derive(Debug, Clone, PartialEq)]
pub enum DstMove {
    /// Existing page number.
    Page(u32),
    /// New page inserted directly after this page number.
    NewPageAfter(u32),
}

/// Destination for `page move <>` (swap).
#[derive(Debug, Clone, PartialEq)]
pub enum DstSwap {
    /// One or more full pages.
    Pages(PagesExpr),
    /// Specific slots on a single page.
    Slots { page: u32, slots: SlotExpr },
}

/// Parsed `page move` command — either a move or a swap.
#[derive(Debug, Clone, PartialEq)]
pub enum PageMoveCmd {
    Move { src: Src, dst: DstMove },
    Swap { left: Src, right: DstSwap },
}

// ── Error types ───────────────────────────────────────────────────────────────

/// Semantic validation errors (checked against the loaded project state).
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    PageNotFound(u32),
    SlotNotFound { page: u32, slot: u32 },
    SwapSamePage(u32),
    CombineSinglePage(u32),
    SplitAtFirstSlot(u32),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PageNotFound(p) => write!(f, "page {p} does not exist"),
            Self::SlotNotFound { page, slot } => {
                write!(f, "slot {slot} does not exist on page {page}")
            }
            Self::SwapSamePage(p) => write!(f, "cannot swap page {p} with itself"),
            Self::CombineSinglePage(p) => {
                write!(f, "combine requires at least two pages, got only page {p}")
            }
            Self::SplitAtFirstSlot(p) => {
                write!(f, "cannot split at first slot (would leave page {p} empty)")
            }
        }
    }
}

/// Top-level error for page commands.
#[derive(Debug)]
pub enum PageMoveError {
    Validation(ValidationError),
    Other(anyhow::Error),
}

impl std::fmt::Display for PageMoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(e) => write!(f, "{e}"),
            Self::Other(e) => write!(f, "{e}"),
        }
    }
}

impl From<anyhow::Error> for PageMoveError {
    fn from(e: anyhow::Error) -> Self {
        Self::Other(e)
    }
}

impl From<ValidationError> for PageMoveError {
    fn from(e: ValidationError) -> Self {
        Self::Validation(e)
    }
}

// ── Result type ───────────────────────────────────────────────────────────────

/// Summary of what a page command changed.
#[derive(Debug)]
pub struct PageMoveResult {
    /// Pages whose photo list changed (need rebuild), 1-based.
    pub pages_modified: Vec<u32>,
    /// Pages that were newly inserted, 1-based.
    pub pages_inserted: Vec<u32>,
    /// Pages that were deleted, 1-based (original numbers before deletion).
    pub pages_deleted: Vec<u32>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Resolve a 1-based page number to a 0-based index, or return ValidationError.
fn page_idx(page: u32, layout: &[LayoutPage]) -> Result<usize, ValidationError> {
    if page == 0 || page as usize > layout.len() {
        return Err(ValidationError::PageNotFound(page));
    }
    Ok(page as usize - 1)
}

/// Resolve slot numbers on a page to 0-based indices and validate they exist.
/// `slots` are 1-based slot numbers.
fn resolve_slots(
    page: u32,
    slot_expr: &SlotExpr,
    layout: &[LayoutPage],
) -> Result<Vec<usize>, ValidationError> {
    let idx = page_idx(page, layout)?;
    let n_slots = layout[idx].photos.len();
    let mut result = Vec::with_capacity(slot_expr.slots.len());
    for &s in &slot_expr.slots {
        if s == 0 || s as usize > n_slots {
            return Err(ValidationError::SlotNotFound { page, slot: s });
        }
        result.push(s as usize - 1);
    }
    Ok(result)
}

/// Collect photo IDs at the given 0-based slot indices on a page.
fn photos_at_slots(layout: &[LayoutPage], page_idx: usize, slot_indices: &[usize]) -> Vec<String> {
    slot_indices
        .iter()
        .map(|&i| layout[page_idx].photos[i].clone())
        .collect()
}

/// Remove photos at given 0-based slot indices from a page (descending order to keep indices stable).
fn remove_slots(layout: &mut [LayoutPage], page_idx: usize, mut slot_indices: Vec<usize>) {
    slot_indices.sort_unstable_by(|a, b| b.cmp(a));
    for i in slot_indices {
        layout[page_idx].photos.remove(i);
        if i < layout[page_idx].slots.len() {
            layout[page_idx].slots.remove(i);
        }
    }
}

// ── execute_unplace ───────────────────────────────────────────────────────────

/// Remove photos from the layout at the given page:slot address.
///
/// Photos are kept in `state.photos` (they become "unplaced").
/// Returns the 1-based page numbers that were modified.
pub fn execute_unplace(
    project_root: &Path,
    page: u32,
    slots: SlotExpr,
) -> Result<PageMoveResult, PageMoveError> {
    let mut mgr = StateManager::open(project_root)?;

    // Validate
    let slot_indices = resolve_slots(page, &slots, &mgr.state.layout)?;
    if slot_indices.is_empty() {
        return Ok(PageMoveResult {
            pages_modified: vec![],
            pages_inserted: vec![],
            pages_deleted: vec![],
        });
    }

    // Execute
    let page_idx_val = page_idx(page, &mgr.state.layout)?;
    remove_slots(&mut mgr.state.layout, page_idx_val, slot_indices);

    mgr.finish(&format!("unplace: page {page}"))?;

    Ok(PageMoveResult {
        pages_modified: vec![page],
        pages_inserted: vec![],
        pages_deleted: vec![],
    })
}

// ── execute_move ──────────────────────────────────────────────────────────────

/// Execute a `page move` command (either Move or Swap variant).
pub fn execute_move(
    project_root: &Path,
    cmd: PageMoveCmd,
) -> Result<PageMoveResult, PageMoveError> {
    match cmd {
        PageMoveCmd::Move { src, dst } => execute_move_to(project_root, src, dst),
        PageMoveCmd::Swap { left, right } => execute_swap_cmd(project_root, left, right),
    }
}

fn execute_move_to(
    project_root: &Path,
    src: Src,
    dst: DstMove,
) -> Result<PageMoveResult, PageMoveError> {
    let mut mgr = StateManager::open(project_root)?;

    // Collect source photo IDs and their origin page indices
    let (photos, _src_page_indices) = collect_src_photos(&src, &mgr.state.layout)?;
    if photos.is_empty() {
        return Ok(PageMoveResult {
            pages_modified: vec![],
            pages_inserted: vec![],
            pages_deleted: vec![],
        });
    }

    // Validate / prepare destination
    let (dst_page_idx, inserted_page) = match &dst {
        DstMove::Page(p) => {
            let idx = page_idx(*p, &mgr.state.layout)?;
            (idx, None)
        }
        DstMove::NewPageAfter(p) => {
            let after_idx = page_idx(*p, &mgr.state.layout)?;
            // Insert new empty page after `after_idx`
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

    // Remove photos from source pages (in reverse order to keep indices stable)
    // We need to re-collect slot indices after potential page insertion
    let src_page_indices_after_insert: Vec<usize> = match &src {
        Src::Pages(pe) => {
            // After possible insert, source page indices may shift if insert was before them
            pe.pages
                .iter()
                .map(|&p| page_idx(p, &mgr.state.layout))
                .collect::<Result<Vec<_>, _>>()?
        }
        Src::Slots { page, slots } => {
            let idx = page_idx(*page, &mgr.state.layout)?;
            let slot_indices = resolve_slots(*page, slots, &mgr.state.layout)?;
            remove_slots(&mut mgr.state.layout, idx, slot_indices);
            // dst_page_idx may have shifted if new page was inserted before it
            // but we already computed dst_page_idx after the insert, so it's fine
            // Add photos to dst
            for photo in &photos {
                mgr.state.layout[dst_page_idx].photos.push(photo.clone());
            }
            let src_page_num = match &src {
                Src::Slots { page, .. } => *page,
                _ => unreachable!(),
            };
            let mut modified = vec![src_page_num, dst_page_idx as u32 + 1];
            modified.sort();
            modified.dedup();
            mgr.finish(&format!("page move: slots from page {src_page_num} -> page"))?;
            return Ok(PageMoveResult {
                pages_modified: modified,
                pages_inserted: inserted_page.map(|_| vec![dst_page_idx as u32 + 1]).unwrap_or_default(),
                pages_deleted: vec![],
            });
        }
    };

    // For Pages variant: remove all photos from source pages
    let mut modified_pages: Vec<u32> = Vec::new();
    for &idx in &src_page_indices_after_insert {
        let page_num = mgr.state.layout[idx].page as u32;
        mgr.state.layout[idx].photos.clear();
        mgr.state.layout[idx].slots.clear();
        modified_pages.push(page_num);
    }

    // Add photos to dst page
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
        pages_inserted: inserted_page.map(|_| vec![dst_page_idx as u32 + 1]).unwrap_or_default(),
        pages_deleted: vec![],
    })
}

fn execute_swap_cmd(
    project_root: &Path,
    left: Src,
    right: DstSwap,
) -> Result<PageMoveResult, PageMoveError> {
    let mut mgr = StateManager::open(project_root)?;

    // Validate both sides and check they're not the same page
    let (left_photos, left_page_idx, left_slot_indices) =
        collect_src_photos_with_indices(&left, &mgr.state.layout)?;
    let (right_photos, right_page_idx, right_slot_indices) =
        collect_dst_swap_photos_with_indices(&right, &mgr.state.layout)?;

    // Check same-page swap (only for single-page cases)
    if let (Some(lp), Some(rp)) = (single_page_of_src(&left), single_page_of_dst_swap(&right)) {
        if lp == rp {
            return Err(ValidationError::SwapSamePage(lp).into());
        }
    }

    // Perform the swap: exchange photo IDs at the given positions
    // Strategy: replace left slots with right photos, right slots with left photos
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
    // Remove left photos (descending)
    let mut left_desc: Vec<usize> = left_slot_indices.to_vec();
    left_desc.sort_unstable_by(|a, b| b.cmp(a));
    for &i in &left_desc {
        layout[left_page_idx].photos.remove(i);
        if i < layout[left_page_idx].slots.len() {
            layout[left_page_idx].slots.remove(i);
        }
    }

    // Insert right photos at left positions
    let insert_at = left_slot_indices.iter().min().copied().unwrap_or(0);
    for (j, photo) in right_photos.iter().enumerate() {
        let pos = (insert_at + j).min(layout[left_page_idx].photos.len());
        layout[left_page_idx].photos.insert(pos, photo.clone());
    }

    // Remove right photos (descending) — note: if same page this gets complex, but we
    // validate different pages above
    let mut right_desc: Vec<usize> = right_slot_indices.to_vec();
    right_desc.sort_unstable_by(|a, b| b.cmp(a));
    for &i in &right_desc {
        layout[right_page_idx].photos.remove(i);
        if i < layout[right_page_idx].slots.len() {
            layout[right_page_idx].slots.remove(i);
        }
    }

    // Insert left photos at right positions
    let insert_at_r = right_slot_indices.iter().min().copied().unwrap_or(0);
    for (j, photo) in left_photos.iter().enumerate() {
        let pos = (insert_at_r + j).min(layout[right_page_idx].photos.len());
        layout[right_page_idx].photos.insert(pos, photo.clone());
    }
}

// ── execute_split ─────────────────────────────────────────────────────────────

/// Split a page at a given slot: photos from `slot` onwards move to a new page after it.
///
/// `page` and `slot` are 1-based.
pub fn execute_split(
    project_root: &Path,
    page: u32,
    slot: u32,
) -> Result<PageMoveResult, PageMoveError> {
    let mut mgr = StateManager::open(project_root)?;

    let idx = page_idx(page, &mgr.state.layout)?;
    let n_photos = mgr.state.layout[idx].photos.len();

    if slot == 0 || slot as usize > n_photos {
        return Err(ValidationError::SlotNotFound { page, slot }.into());
    }
    if slot == 1 {
        return Err(ValidationError::SplitAtFirstSlot(page).into());
    }

    // Photos from slot onwards (0-based: slot-1..)
    let split_at = slot as usize - 1;
    let moved_photos: Vec<String> = mgr.state.layout[idx].photos[split_at..].to_vec();
    let moved_slots: Vec<_> = if split_at < mgr.state.layout[idx].slots.len() {
        mgr.state.layout[idx].slots[split_at..].to_vec()
    } else {
        vec![]
    };

    // Truncate source page
    mgr.state.layout[idx].photos.truncate(split_at);
    mgr.state.layout[idx].slots.truncate(split_at);

    // Insert new page after idx
    let new_idx = idx + 1;
    mgr.state.layout.insert(
        new_idx,
        LayoutPage {
            page: (new_idx + 1) as usize, // will be renumbered
            photos: moved_photos,
            slots: moved_slots,
        },
    );

    let new_page_num = new_idx as u32 + 1;
    mgr.finish(&format!("page split: page {page} at slot {slot}"))?;

    Ok(PageMoveResult {
        pages_modified: vec![page],
        pages_inserted: vec![new_page_num],
        pages_deleted: vec![],
    })
}

// ── execute_combine ───────────────────────────────────────────────────────────

/// Combine all given pages onto the first page and delete the rest.
///
/// Pages in `pages_expr` must be 1-based. At least two pages required.
pub fn execute_combine(
    project_root: &Path,
    pages_expr: PagesExpr,
) -> Result<PageMoveResult, PageMoveError> {
    let mut mgr = StateManager::open(project_root)?;

    if pages_expr.pages.len() < 2 {
        let p = pages_expr.pages.first().copied().unwrap_or(0);
        return Err(ValidationError::CombineSinglePage(p).into());
    }

    // Validate all pages exist
    for &p in &pages_expr.pages {
        page_idx(p, &mgr.state.layout)?;
    }

    let first_page = pages_expr.pages[0];
    let first_idx = page_idx(first_page, &mgr.state.layout)?;

    // Collect photos from all other pages
    let mut extra_photos: Vec<String> = Vec::new();
    let other_pages: Vec<u32> = pages_expr.pages[1..].to_vec();
    for &p in &other_pages {
        let idx = page_idx(p, &mgr.state.layout)?;
        extra_photos.extend(mgr.state.layout[idx].photos.clone());
    }

    // Add to first page
    mgr.state.layout[first_idx]
        .photos
        .extend(extra_photos);
    mgr.state.layout[first_idx].slots.clear(); // needs rebuild

    // Delete other pages (sorted descending by index to keep indices stable)
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

// ── Helpers for collecting photos ─────────────────────────────────────────────

fn collect_src_photos(
    src: &Src,
    layout: &[LayoutPage],
) -> Result<(Vec<String>, Vec<usize>), ValidationError> {
    match src {
        Src::Pages(pe) => {
            let mut photos = Vec::new();
            let mut indices = Vec::new();
            for &p in &pe.pages {
                let idx = page_idx(p, layout)?;
                photos.extend(layout[idx].photos.clone());
                indices.push(idx);
            }
            Ok((photos, indices))
        }
        Src::Slots { page, slots } => {
            let idx = page_idx(*page, layout)?;
            let slot_indices = resolve_slots(*page, slots, layout)?;
            let photos = photos_at_slots(layout, idx, &slot_indices);
            Ok((photos, vec![idx]))
        }
    }
}

/// Returns (photos, page_idx, slot_indices_within_page).
/// For Pages variant, slot_indices contains all indices [0..n_photos).
fn collect_src_photos_with_indices(
    src: &Src,
    layout: &[LayoutPage],
) -> Result<(Vec<String>, usize, Vec<usize>), ValidationError> {
    match src {
        Src::Pages(pe) => {
            if pe.pages.len() != 1 {
                // For multi-page swap, use the first page (this is a simplification
                // for the swap operation — multi-page swaps operate page by page)
                let p = pe.pages[0];
                let idx = page_idx(p, layout)?;
                let all_slots: Vec<usize> = (0..layout[idx].photos.len()).collect();
                let photos = layout[idx].photos.clone();
                Ok((photos, idx, all_slots))
            } else {
                let p = pe.pages[0];
                let idx = page_idx(p, layout)?;
                let all_slots: Vec<usize> = (0..layout[idx].photos.len()).collect();
                let photos = layout[idx].photos.clone();
                Ok((photos, idx, all_slots))
            }
        }
        Src::Slots { page, slots } => {
            let idx = page_idx(*page, layout)?;
            let slot_indices = resolve_slots(*page, slots, layout)?;
            let photos = photos_at_slots(layout, idx, &slot_indices);
            Ok((photos, idx, slot_indices))
        }
    }
}

fn collect_dst_swap_photos_with_indices(
    dst: &DstSwap,
    layout: &[LayoutPage],
) -> Result<(Vec<String>, usize, Vec<usize>), ValidationError> {
    match dst {
        DstSwap::Pages(pe) => {
            let p = pe.pages[0];
            let idx = page_idx(p, layout)?;
            let all_slots: Vec<usize> = (0..layout[idx].photos.len()).collect();
            let photos = layout[idx].photos.clone();
            Ok((photos, idx, all_slots))
        }
        DstSwap::Slots { page, slots } => {
            let idx = page_idx(*page, layout)?;
            let slot_indices = resolve_slots(*page, slots, layout)?;
            let photos = photos_at_slots(layout, idx, &slot_indices);
            Ok((photos, idx, slot_indices))
        }
    }
}

fn single_page_of_src(src: &Src) -> Option<u32> {
    match src {
        Src::Pages(pe) if pe.pages.len() == 1 => Some(pe.pages[0]),
        Src::Slots { page, .. } => Some(*page),
        _ => None,
    }
}

fn single_page_of_dst_swap(dst: &DstSwap) -> Option<u32> {
    match dst {
        DstSwap::Pages(pe) if pe.pages.len() == 1 => Some(pe.pages[0]),
        DstSwap::Slots { page, .. } => Some(*page),
        _ => None,
    }
}

fn format_src_desc(src: &Src) -> String {
    match src {
        Src::Pages(pe) => format_pages_list(&pe.pages),
        Src::Slots { page, slots } => {
            let slot_list: Vec<String> = slots.slots.iter().map(|s| s.to_string()).collect();
            format!("page {}:{}", page, slot_list.join(","))
        }
    }
}

fn format_pages_list(pages: &[u32]) -> String {
    let list: Vec<String> = pages.iter().map(|p| p.to_string()).collect();
    list.join(",")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::{
        BookConfig, BookLayoutSolverConfig, LayoutPage, PhotoFile, PhotoGroup, ProjectConfig,
        ProjectState, Slot,
    };
    use crate::git;
    use tempfile::TempDir;

    fn make_slot() -> Slot {
        Slot {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: 100.0,
            height_mm: 80.0,
        }
    }

    fn make_state_with_layout(pages: Vec<Vec<&str>>) -> ProjectState {
        let layout: Vec<LayoutPage> = pages
            .into_iter()
            .enumerate()
            .map(|(i, photos)| LayoutPage {
                page: i + 1,
                photos: photos.iter().map(|s| s.to_string()).collect(),
                slots: (0..photos.len()).map(|_| make_slot()).collect(),
            })
            .collect();

        ProjectState {
            config: ProjectConfig {
                book: BookConfig {
                    title: "Test".to_owned(),
                    page_width_mm: 420.0,
                    page_height_mm: 297.0,
                    bleed_mm: 3.0,
                    margin_mm: 10.0,
                    gap_mm: 5.0,
                    bleed_threshold_mm: 3.0,
                    dpi: 300.0,
                },
                page_layout_solver: Default::default(),
                preview: Default::default(),
                book_layout_solver: BookLayoutSolverConfig::default(),
            },
            photos: vec![PhotoGroup {
                group: "Test".to_owned(),
                sort_key: "2024-01-01".to_owned(),
                files: (0..10)
                    .map(|i| PhotoFile {
                        id: format!("p{i}.jpg"),
                        source: format!("/photos/p{i}.jpg"),
                        timestamp: "2024-01-01T00:00:00Z".parse().unwrap(),
                        width_px: 4000,
                        height_px: 3000,
                        area_weight: 1.0,
                        hash: String::new(),
                    })
                    .collect(),
            }],
            layout,
        }
    }

    fn setup_repo(tmp: &TempDir, state: &ProjectState) {
        let repo = git::init_repo(tmp.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        drop(config);

        std::fs::write(
            tmp.path().join(".gitignore"),
            ".fotobuch/\n*.pdf\nlog*\n",
        )
        .unwrap();
        state.save(&tmp.path().join("urlaub.yaml")).unwrap();
        git::stage_and_commit(&repo, &[".gitignore", "urlaub.yaml"], "init").unwrap();
        git::create_branch(&repo, "fotobuch/urlaub").unwrap();
    }

    // ── page_idx ──────────────────────────────────────────────────────────────

    #[test]
    fn test_page_idx_valid() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"], vec!["p1.jpg"]]);
        assert_eq!(page_idx(1, &state.layout).unwrap(), 0);
        assert_eq!(page_idx(2, &state.layout).unwrap(), 1);
    }

    #[test]
    fn test_page_idx_out_of_range() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"]]);
        assert_eq!(
            page_idx(2, &state.layout),
            Err(ValidationError::PageNotFound(2))
        );
        assert_eq!(
            page_idx(0, &state.layout),
            Err(ValidationError::PageNotFound(0))
        );
    }

    // ── resolve_slots ─────────────────────────────────────────────────────────

    #[test]
    fn test_resolve_slots_valid() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg", "p2.jpg"]]);
        let slots = SlotExpr::from_range(1, 3);
        let indices = resolve_slots(1, &slots, &state.layout).unwrap();
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_resolve_slots_out_of_range() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"]]);
        let slots = SlotExpr::single(3);
        assert_eq!(
            resolve_slots(1, &slots, &state.layout),
            Err(ValidationError::SlotNotFound { page: 1, slot: 3 })
        );
    }

    // ── PagesExpr / SlotExpr constructors ────────────────────────────────────

    #[test]
    fn test_pages_expr_from_range() {
        let pe = PagesExpr::from_range(3, 5);
        assert_eq!(pe.pages, vec![3, 4, 5]);
    }

    #[test]
    fn test_slot_expr_from_range() {
        let se = SlotExpr::from_range(2, 5);
        assert_eq!(se.slots, vec![2, 3, 4, 5]);
    }

    // ── execute_unplace ───────────────────────────────────────────────────────

    #[test]
    fn test_execute_unplace_removes_photo() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg", "p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result = execute_unplace(tmp.path(), 1, SlotExpr::single(2)).unwrap();
        assert_eq!(result.pages_modified, vec![1]);

        let mgr = StateManager::open(tmp.path()).unwrap();
        let page = &mgr.state.layout[0];
        assert_eq!(page.photos, vec!["p0.jpg", "p2.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_unplace_invalid_slot() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result = execute_unplace(tmp.path(), 1, SlotExpr::single(5));
        assert!(matches!(
            result,
            Err(PageMoveError::Validation(ValidationError::SlotNotFound {
                page: 1,
                slot: 5
            }))
        ));
    }

    #[test]
    fn test_execute_unplace_invalid_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result = execute_unplace(tmp.path(), 99, SlotExpr::single(1));
        assert!(matches!(
            result,
            Err(PageMoveError::Validation(ValidationError::PageNotFound(99)))
        ));
    }

    // ── execute_move (Move variant) ───────────────────────────────────────────

    #[test]
    fn test_execute_move_pages_to_page() {
        // Move all photos from page 2 to page 1
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
        // Page 2 should be empty
        let page2 = &mgr.state.layout[1];
        assert!(page2.photos.is_empty());
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_move_to_new_page() {
        // Move all photos from page 1 to a new page after page 1
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
        // Should now have 3 pages
        assert_eq!(mgr.state.layout.len(), 3);
        mgr.finish("test: noop").unwrap();
    }

    // ── execute_split ─────────────────────────────────────────────────────────

    #[test]
    fn test_execute_split_creates_new_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg", "p2.jpg", "p3.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result = execute_split(tmp.path(), 1, 3).unwrap();
        assert!(!result.pages_inserted.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state.layout.len(), 2);
        assert_eq!(mgr.state.layout[0].photos, vec!["p0.jpg", "p1.jpg"]);
        assert_eq!(mgr.state.layout[1].photos, vec!["p2.jpg", "p3.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_split_at_first_slot_is_error() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result = execute_split(tmp.path(), 1, 1);
        assert!(matches!(
            result,
            Err(PageMoveError::Validation(ValidationError::SplitAtFirstSlot(1)))
        ));
    }

    // ── execute_combine ───────────────────────────────────────────────────────

    #[test]
    fn test_execute_combine_merges_pages() {
        let state = make_state_with_layout(vec![
            vec!["p0.jpg", "p1.jpg"],
            vec!["p2.jpg"],
            vec!["p3.jpg", "p4.jpg"],
        ]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let pages = PagesExpr::from_range(1, 3);
        let result = execute_combine(tmp.path(), pages).unwrap();
        assert_eq!(result.pages_deleted, vec![2, 3]);

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

        let pages = PagesExpr::single(1);
        let result = execute_combine(tmp.path(), pages);
        assert!(matches!(
            result,
            Err(PageMoveError::Validation(ValidationError::CombineSinglePage(1)))
        ));
    }

    // ── ValidationError display ───────────────────────────────────────────────

    #[test]
    fn test_validation_error_display() {
        assert_eq!(
            ValidationError::PageNotFound(5).to_string(),
            "page 5 does not exist"
        );
        assert_eq!(
            ValidationError::SlotNotFound { page: 3, slot: 7 }.to_string(),
            "slot 7 does not exist on page 3"
        );
        assert_eq!(
            ValidationError::SplitAtFirstSlot(2).to_string(),
            "cannot split at first slot (would leave page 2 empty)"
        );
    }
}
