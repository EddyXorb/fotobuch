use super::core::rebuild_single_page::rebuild_single_page;
use super::cover::{build_cover_page, split_cover_photos, update_cover_page};
use super::helpers::{
    CommitMode, PdfTarget, RenderContext, build_photo_index, collect_photos_as_groups, render_pdf,
    update_preview_cache,
};
use super::{BuildConfig, BuildOptions, BuildResult, DpiWarning};
use crate::cache::final_cache;
use crate::commands::CommandOutput;
use crate::dto_models::{BookLayoutSolverConfig, LayoutPage, PageMode, PhotoGroup};
use crate::solver::{Request, RequestType, run_solver};
use crate::state_manager::{StateManager, renumber_pages};
use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use tracing::{info, warn};

/// Scope of a rebuild operation (input to `BuildPlan::from_rebuild_scope`).
///
/// All page references use **0-based array indices** (position in `layout[]`).
/// Cover page (when active) is always at index 0.
#[derive(Debug, Clone)]
pub enum RebuildScope {
    /// Rebuild all pages (like first build, but always commits the result).
    All,
    /// Rebuild single page.
    /// `page_idx` is a 0-based index into `layout[]`.
    SinglePage(usize),
    /// Rebuild page range with optional flexibility.
    /// `start` and `end` are both 0-based inclusive indices into `layout[]`.
    Range {
        start: usize,
        end: usize,
        /// Allow page count to vary by +/- N (default: 0)
        flex: usize,
    },
}

/// Describes the layout-change strategy for one build or rebuild invocation.
#[derive(Debug, Clone)]
pub enum BuildPlan {
    /// First build or full rebuild: all photos → all pages via the book-layout solver.
    Full { always_commit: bool },
    /// Incremental build: re-solve only pages whose photos changed since last commit.
    Incremental { pages: Option<Vec<usize>> },
    /// Re-solve a single page with the GA solver.
    Page(usize),
    /// Re-solve a page range with the book-layout solver.
    Range {
        start: usize,
        end: usize,
        flex: usize,
    },
    /// Release build: generate final high-quality PDF, no re-solving.
    Release { force: bool },
}

impl BuildPlan {
    /// Constructs a plan from a `build` command config.
    pub fn from_build_config(mgr: &StateManager, config: &BuildConfig) -> Result<Self> {
        if config.release {
            if config.pages.is_some() {
                anyhow::bail!("--pages is not allowed with release (must build entire book)");
            }
            return Ok(BuildPlan::Release {
                force: config.force,
            });
        }
        if mgr.state.layout.is_empty() {
            Ok(BuildPlan::Full {
                always_commit: false,
            })
        } else {
            Ok(BuildPlan::Incremental {
                pages: config.pages.clone(),
            })
        }
    }

    /// Constructs a plan from a `rebuild` command scope, validating the scope against the layout.
    pub fn from_rebuild_scope(mgr: &StateManager, scope: RebuildScope) -> Result<Self> {
        if !matches!(scope, RebuildScope::All) && mgr.state.layout.is_empty() {
            anyhow::bail!(
                "No layout exists. Run `fotobuch build` first, \
                 or use `fotobuch rebuild` (without arguments) for a full rebuild."
            );
        }
        if let RebuildScope::Range { start, end, .. } = scope
            && (start > end || end >= mgr.state.layout.len())
        {
            anyhow::bail!(
                "Invalid page range {}-{} (layout has {} pages, indices 0..{})",
                start,
                end,
                mgr.state.layout.len(),
                mgr.state.layout.len().saturating_sub(1),
            );
        }
        if let RebuildScope::SinglePage(idx) = scope
            && idx >= mgr.state.layout.len()
        {
            anyhow::bail!(
                "Invalid page index {} (layout has {} pages, indices 0..{})",
                idx,
                mgr.state.layout.len(),
                mgr.state.layout.len().saturating_sub(1),
            );
        }
        Ok(match scope {
            RebuildScope::All => BuildPlan::Full {
                always_commit: true,
            },
            RebuildScope::SinglePage(idx) => BuildPlan::Page(idx),
            RebuildScope::Range { start, end, flex } => BuildPlan::Range { start, end, flex },
        })
    }
    /// Runs the layout solver for this variant and updates `mgr.state.layout`.
    ///
    /// Pure layout step: no cache update, no commit, no PDF.
    /// Returns 0-based indices of pages that were modified.
    pub fn resolve_layout(&self, mgr: &mut StateManager) -> Result<Vec<usize>> {
        match self {
            BuildPlan::Full { .. } => resolve_whole_book(mgr),
            BuildPlan::Incremental { pages } => resolve_outdated_pages(mgr, pages.as_deref()),
            BuildPlan::Page(idx) => resolve_single_page(mgr, *idx),
            BuildPlan::Range { start, end, flex } => resolve_range(mgr, *start, *end, *flex),
            BuildPlan::Release { .. } => Ok(Vec::new()),
        }
    }

    /// Executes the full build pipeline for this plan.
    ///
    /// Steps: cache refresh → resolve layout → renumber → commit → PDF → BuildResult.
    pub fn run(
        self,
        mut mgr: StateManager,
        project_root: &Path,
        opts: BuildOptions,
    ) -> Result<CommandOutput<BuildResult>> {
        if let BuildPlan::Release { force } = self {
            validate_release(&mgr, force)?;
        }

        // 1. Update image cache
        let (images_processed, dpi_warnings) = refresh_cache(&self, &mut mgr, &opts)?;

        // 2. Resolve layout (pure)
        let changed_pages = self.resolve_layout(&mut mgr)?;

        // 3. Renumber pages
        let has_cover = mgr.state.has_cover();
        renumber_pages(&mut mgr.state.layout, has_cover);

        let ctx = RenderContext::capture(&mgr);
        let page_count = mgr.state.layout.len();
        let total_photos: usize = mgr.state.layout.iter().map(|p| p.photos.len()).sum();

        // 4. Commit
        let msg = commit_message(&self, &changed_pages, page_count, total_photos);
        let changed_state = match commit_mode(&self) {
            CommitMode::Always => mgr.finish_always(&msg)?,
            CommitMode::Auto => mgr.finish(&msg)?,
        };

        // 5. PDF
        let pdf_path = render_pdf(
            project_root,
            &ctx.project_name,
            ctx.bleed_mm,
            pdf_target(&self),
            effective_skip_pdf(&self, &opts),
        )?;

        Ok(CommandOutput {
            result: BuildResult {
                pdf_path,
                pages_rebuilt: changed_pages.clone(),
                images_processed,
                dpi_warnings,
                nothing_to_do: changed_pages.is_empty()
                    && !matches!(self, BuildPlan::Release { .. } | BuildPlan::Full { .. }),
            },
            changed_state,
        })
    }
}

// ── pipeline helpers ─────────────────────────────────────────────────────────

fn validate_release(mgr: &StateManager, force: bool) -> Result<()> {
    if mgr.state.layout.is_empty() {
        anyhow::bail!("No layout found. Run `fotobuch build` first to generate layout.");
    }
    if !force {
        let changed: Vec<_> = mgr
            .outdated_pages_indices()
            .into_iter()
            .filter(|i| !mgr.state.config.book.cover.active || *i != 0)
            .collect();
        if !changed.is_empty() {
            anyhow::bail!(
                "Layout has changes since last build. Changed pages: {:?}. \
                 Run `fotobuch build` first or use `fotobuch build release --force`.",
                changed
            );
        }
    }
    Ok(())
}

fn refresh_cache(
    plan: &BuildPlan,
    mgr: &mut StateManager,
    opts: &BuildOptions,
) -> Result<(usize, Vec<DpiWarning>)> {
    if let BuildPlan::Release { .. } = plan {
        let dpi = mgr.state.config.book.dpi;
        info!("Release build: generating final PDF at {:.0} DPI...", dpi);
        let progress = AtomicUsize::new(0);
        let final_cache_dir = mgr.final_cache_dir();
        let result = final_cache::build_final_cache(&mut mgr.state, &final_cache_dir, &progress)?;
        info!(
            "Final cache: {} images generated, {} DPI warnings",
            result.created,
            result.dpi_warnings.len()
        );
        if !result.dpi_warnings.is_empty() {
            warn!(
                "\nWARNING: Some photos will be displayed below {:.0} DPI:",
                dpi
            );
            for w in &result.dpi_warnings {
                warn!(
                    "  Page {}: {} - {:.2} DPI ({}x{} px in {:.1}x{:.1} mm slot)",
                    w.page,
                    w.photo_id,
                    w.actual_dpi,
                    w.original_px.0,
                    w.original_px.1,
                    w.slot_mm.0,
                    w.slot_mm.1
                );
            }
        }
        return Ok((result.created, result.dpi_warnings));
    }

    if opts.skip_cache_update {
        return Ok((0, vec![]));
    }
    let result = update_preview_cache(mgr)?;
    Ok((result.created, vec![]))
}

fn commit_mode(plan: &BuildPlan) -> CommitMode {
    match plan {
        BuildPlan::Full {
            always_commit: true,
        } => CommitMode::Always,
        BuildPlan::Full { .. } | BuildPlan::Incremental { .. } => CommitMode::Auto,
        BuildPlan::Page(_) | BuildPlan::Range { .. } | BuildPlan::Release { .. } => {
            CommitMode::Always
        }
    }
}

fn pdf_target(plan: &BuildPlan) -> PdfTarget {
    if matches!(plan, BuildPlan::Release { .. }) {
        PdfTarget::Final
    } else {
        PdfTarget::Preview
    }
}

fn effective_skip_pdf(plan: &BuildPlan, opts: &BuildOptions) -> bool {
    if matches!(plan, BuildPlan::Release { .. }) {
        false
    } else {
        opts.skip_pdf
    }
}

fn commit_message(
    plan: &BuildPlan,
    changed_pages: &[usize],
    page_count: usize,
    total_photos: usize,
) -> String {
    match plan {
        BuildPlan::Full {
            always_commit: true,
        } => {
            format!("rebuild: {} photos redistributed", total_photos)
        }
        BuildPlan::Full { .. } => "build: initial layout".to_string(),
        BuildPlan::Incremental { .. } => {
            format!("build: {} page(s) rebuilt", changed_pages.len())
        }
        BuildPlan::Page(idx) => format!("rebuild: page {}", idx),
        BuildPlan::Range { start, end, .. } => format!("rebuild: pages {}-{}", start, end),
        BuildPlan::Release { .. } => {
            format!("release: {} pages, {} photos", page_count, total_photos)
        }
    }
}

// ── resolve_* handlers ────────────────────────────────────────────────────────

fn resolve_whole_book(mgr: &mut StateManager) -> Result<Vec<usize>> {
    let layout_len = mgr.state.layout.len();
    // For an existing layout with an active cover, skip page 0 so the cover is not
    // redistributed (use `rebuild --page 0` to rebuild it explicitly).
    if layout_len > 0 && mgr.state.has_cover() {
        let effective_start = skip_cover_if_needed(true, 0, layout_len - 1)?;
        let groups = collect_photos_as_groups(&mgr.state, effective_start, layout_len);
        return solve_multipage_layout(mgr, &groups, Some((effective_start, layout_len)), None);
    }
    let groups = mgr.state.photos.clone();
    solve_multipage_layout(mgr, &groups, None, None)
}

fn resolve_outdated_pages(
    mgr: &mut StateManager,
    page_filter: Option<&[usize]>,
) -> Result<Vec<usize>> {
    let mut pages = mgr.outdated_pages_indices();
    if let Some(filter) = page_filter {
        pages.retain(|p| filter.contains(p));
    }
    let photo_index = build_photo_index(&mgr.state.photos);
    for &idx in &pages {
        if mgr.state.layout[idx].mode != PageMode::Manual {
            rebuild_single_page(&mut mgr.state, idx, &photo_index)?;
        }
    }
    Ok(pages)
}

fn resolve_single_page(mgr: &mut StateManager, idx: usize) -> Result<Vec<usize>> {
    if idx >= mgr.state.layout.len() {
        anyhow::bail!(
            "Page {} does not exist (layout has {} pages)",
            idx,
            mgr.state.layout.len()
        );
    }
    if mgr.state.layout[idx].mode == PageMode::Manual {
        anyhow::bail!(
            "Cannot rebuild page {}: page is in manual mode. \
             Use `page mode {} a` to switch to auto mode first.",
            idx,
            idx
        );
    }
    let photo_index = build_photo_index(&mgr.state.photos);
    rebuild_single_page(&mut mgr.state, idx, &photo_index)?;
    Ok(vec![idx])
}

fn resolve_range(
    mgr: &mut StateManager,
    start: usize,
    end: usize,
    flex: usize,
) -> Result<Vec<usize>> {
    let effective_start = skip_cover_if_needed(mgr.state.has_cover(), start, end)?;
    let groups = collect_photos_as_groups(&mgr.state, effective_start, end + 1);
    let n = end - effective_start + 1;
    let custom_config = BookLayoutSolverConfig {
        page_min: n.saturating_sub(flex).max(1),
        page_max: n + flex,
        page_target: n,
        ..mgr.state.config.book_layout_solver.clone()
    };
    solve_multipage_layout(
        mgr,
        &groups,
        Some((effective_start, end + 1)),
        Some(custom_config),
    )
}

// ── shared helpers ────────────────────────────────────────────────────────────

/// If cover is active and `start` is 0, skips the cover and returns effective start = 1.
/// Emits a warning in that case. Returns `Err` if the resulting range would be empty.
pub(super) fn skip_cover_if_needed(has_cover: bool, start: usize, end: usize) -> Result<usize> {
    if !has_cover || start != 0 {
        return Ok(start);
    }
    warn!(
        "Cover page (index 0) is excluded from this rebuild. \
         Use `rebuild --page 0` to rebuild it explicitly."
    );
    if end == 0 {
        anyhow::bail!(
            "Range 0-0 contains only the cover page. \
             Use `rebuild --page 0` to rebuild it explicitly."
        );
    }
    Ok(1)
}

fn solve_multipage_layout(
    mgr: &mut StateManager,
    groups: &[PhotoGroup],
    range: Option<(usize, usize)>,
    custom_config: Option<BookLayoutSolverConfig>,
) -> Result<Vec<usize>> {
    let solver_config =
        custom_config.unwrap_or_else(|| mgr.state.config.book_layout_solver.clone());
    let ga_config = mgr.state.config.page_layout_solver.clone();
    let book_config = mgr.state.config.book.clone();
    let cover_cfg = &book_config.cover;

    let is_structured_cover = range.is_none() && cover_cfg.active && !cover_cfg.mode.is_free();
    let (cover_files_opt, inner_groups) = if is_structured_cover {
        let n = cover_cfg.mode.required_slots().unwrap();
        let (cover_files, remaining) = split_cover_photos(groups, n);
        (Some(cover_files), remaining)
    } else {
        (None, groups.to_vec())
    };

    let (manual_snapshots, filtered_groups) =
        extract_manual_pages(&mgr.state.layout, &inner_groups, range);

    let mut new_pages = run_solver(&Request {
        request_type: RequestType::MultiPage,
        groups: &filtered_groups,
        config: &solver_config,
        ga_config: &ga_config,
        canvas_config: &book_config,
    })?;

    if let Some(cover_files) = cover_files_opt {
        let inner_count = new_pages.len();
        let cover_page = build_cover_page(cover_cfg, cover_files, inner_count)?;
        new_pages.insert(0, cover_page);
    }

    let range_start = range.map(|(s, _)| s).unwrap_or(0);
    for (orig_abs_idx, manual_page) in manual_snapshots {
        let insert_at = orig_abs_idx
            .saturating_sub(range_start)
            .min(new_pages.len());
        new_pages.insert(insert_at, manual_page);
    }

    let affected = if let Some((start, end)) = range {
        let indices: Vec<usize> = (start..start + new_pages.len()).collect();
        mgr.state.layout.splice(start..end, new_pages);
        indices
    } else {
        let indices: Vec<usize> = (0..new_pages.len()).collect();
        mgr.state.layout = new_pages;
        indices
    };

    if range.is_none_or(|r| r.0 == 0)
        && book_config.cover.active
        && book_config.cover.mode.is_free()
    {
        let photo_index = build_photo_index(&mgr.state.photos);
        update_cover_page(&mut mgr.state, &photo_index)?;
    }

    Ok(affected)
}

/// Extracts manual pages from the layout range and filters their photos from the groups.
///
/// Returns `(snapshots, filtered_groups)` where snapshots are `(original_absolute_index, page)`.
pub(super) fn extract_manual_pages(
    layout: &[LayoutPage],
    groups: &[PhotoGroup],
    range: Option<(usize, usize)>,
) -> (Vec<(usize, LayoutPage)>, Vec<PhotoGroup>) {
    let (range_start, range_end) = match range {
        Some((s, e)) => (s, e),
        None => (0, layout.len()),
    };

    let snapshots: Vec<(usize, LayoutPage)> = layout[range_start..range_end.min(layout.len())]
        .iter()
        .enumerate()
        .filter(|(_, p)| p.mode == PageMode::Manual)
        .map(|(i, p)| (range_start + i, p.clone()))
        .collect();

    if snapshots.is_empty() {
        return (snapshots, groups.to_vec());
    }

    let manual_ids: HashSet<&str> = snapshots
        .iter()
        .flat_map(|(_, p)| p.photos.iter().map(String::as_str))
        .collect();

    let filtered: Vec<PhotoGroup> = groups
        .iter()
        .filter_map(|g| {
            let files: Vec<_> = g
                .files
                .iter()
                .filter(|f| !manual_ids.contains(f.id.as_str()))
                .cloned()
                .collect();
            if files.is_empty() {
                None
            } else {
                Some(PhotoGroup {
                    group: g.group.clone(),
                    sort_key: g.sort_key.clone(),
                    files,
                })
            }
        })
        .collect();

    (snapshots, filtered)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::PhotoFile;
    use chrono::Utc;

    fn make_file(id: &str, w: u32, h: u32) -> PhotoFile {
        PhotoFile {
            id: id.to_string(),
            source: format!("/photos/{id}.jpg"),
            width_px: w,
            height_px: h,
            area_weight: 1.0,
            timestamp: Utc::now(),
            hash: "abc".to_string(),
        }
    }

    fn make_group(name: &str, ids: &[(&str, u32, u32)]) -> PhotoGroup {
        PhotoGroup {
            group: name.to_string(),
            sort_key: name.to_string(),
            files: ids.iter().map(|(id, w, h)| make_file(id, *w, *h)).collect(),
        }
    }

    fn make_auto_page(idx: usize, ids: &[&str]) -> LayoutPage {
        LayoutPage {
            page: idx,
            photos: ids.iter().map(|s| s.to_string()).collect(),
            slots: vec![],
            mode: PageMode::Auto,
        }
    }

    fn make_manual_page(idx: usize, ids: &[&str]) -> LayoutPage {
        LayoutPage {
            page: idx,
            photos: ids.iter().map(|s| s.to_string()).collect(),
            slots: vec![],
            mode: PageMode::Manual,
        }
    }

    // ── skip_cover_if_needed ──────────────────────────────────────────────────

    #[test]
    fn skip_cover_no_cover_returns_start_unchanged() {
        assert_eq!(skip_cover_if_needed(false, 0, 3).unwrap(), 0);
        assert_eq!(skip_cover_if_needed(false, 2, 5).unwrap(), 2);
    }

    #[test]
    fn skip_cover_has_cover_nonzero_start_unchanged() {
        assert_eq!(skip_cover_if_needed(true, 2, 5).unwrap(), 2);
    }

    #[test]
    fn skip_cover_has_cover_start_zero_returns_one() {
        assert_eq!(skip_cover_if_needed(true, 0, 3).unwrap(), 1);
    }

    #[test]
    fn skip_cover_range_zero_zero_errors() {
        assert!(skip_cover_if_needed(true, 0, 0).is_err());
    }

    // ── extract_manual_pages ──────────────────────────────────────────────────

    #[test]
    fn extract_manual_pages_preserves_manual_pages() {
        let layout = vec![
            make_auto_page(0, &["a", "b"]),
            make_manual_page(1, &["m1", "m2"]),
            make_auto_page(2, &["c"]),
        ];
        let groups = vec![
            make_group("g1", &[("a", 3, 2), ("b", 4, 3)]),
            make_group("g_manual", &[("m1", 1, 1), ("m2", 1, 1)]),
            make_group("g2", &[("c", 2, 3)]),
        ];

        let (snapshots, filtered) = extract_manual_pages(&layout, &groups, None);
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].0, 1);
        assert_eq!(snapshots[0].1.photos, vec!["m1", "m2"]);
        let all_ids: Vec<_> = filtered
            .iter()
            .flat_map(|g| g.files.iter().map(|f| f.id.as_str()))
            .collect();
        assert!(!all_ids.contains(&"m1"));
        assert!(!all_ids.contains(&"m2"));
        assert!(all_ids.contains(&"a"));
        assert!(all_ids.contains(&"c"));
    }

    #[test]
    fn extract_manual_pages_no_manual_returns_unchanged() {
        let layout = vec![make_auto_page(0, &["a"]), make_auto_page(1, &["b"])];
        let groups = vec![make_group("g", &[("a", 1, 1), ("b", 1, 1)])];
        let (snapshots, filtered) = extract_manual_pages(&layout, &groups, None);
        assert!(snapshots.is_empty());
        assert_eq!(filtered.len(), groups.len());
    }

    #[test]
    fn extract_manual_pages_range_only_within_range() {
        let layout = vec![
            make_auto_page(0, &["a"]),
            make_manual_page(1, &["m1"]),
            make_manual_page(2, &["m2"]),
            make_auto_page(3, &["b"]),
        ];
        let groups = vec![make_group("g", &[("m1", 1, 1), ("m2", 1, 1), ("b", 2, 3)])];
        let (snapshots, filtered) = extract_manual_pages(&layout, &groups, Some((2, 4)));
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].0, 2);
        let ids: Vec<_> = filtered
            .iter()
            .flat_map(|g| g.files.iter().map(|f| f.id.as_str()))
            .collect();
        assert!(!ids.contains(&"m2"));
        assert!(ids.contains(&"b"));
        assert!(ids.contains(&"m1"));
    }

    // ── BuildPlan variant construction ────────────────────────────────────────

    #[test]
    fn build_plan_variants_are_constructible() {
        let _ = BuildPlan::Full {
            always_commit: false,
        };
        let _ = BuildPlan::Full {
            always_commit: true,
        };
        let _ = BuildPlan::Incremental { pages: None };
        let _ = BuildPlan::Incremental {
            pages: Some(vec![0, 2]),
        };
        let _ = BuildPlan::Page(3);
        let _ = BuildPlan::Range {
            start: 1,
            end: 4,
            flex: 1,
        };
        let _ = BuildPlan::Release { force: false };
        let _ = BuildPlan::Release { force: true };
    }
}
