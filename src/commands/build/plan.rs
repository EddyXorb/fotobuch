use super::helpers::{CommitMode, PdfTarget, RenderContext, render_pdf, update_preview_cache};
use super::build_layout::{build_full_book, build_outdated_pages, build_page, build_page_range};
use super::{BuildResult, DpiWarning};
use crate::cache::final_cache;
use crate::commands::CommandOutput;
use crate::state_manager::{StateManager, renumber_pages};
use anyhow::Result;
use std::sync::atomic::AtomicUsize;
use tracing::{info, warn};

/// Describes the layout-change strategy for one build or rebuild invocation.
#[derive(Debug, Clone)]
pub enum BuildPlan {
    /// Automatic: empty layout → solve whole book; existing layout → incremental (outdated pages only).
    Auto { pages: Option<Vec<usize>> },
    /// Full rebuild: solve all photos → all pages, always commits.
    All,
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
    /// Runs the layout solver for this variant and updates `mgr.state.layout`.
    ///
    /// Pure layout step: no cache update, no commit, no PDF.
    /// Returns 0-based indices of pages that were modified.
    pub fn build_layout(&self, mgr: &mut StateManager) -> Result<Vec<usize>> {
        match self {
            BuildPlan::Auto { pages } => {
                if mgr.state.layout.is_empty() {
                    build_full_book(mgr)
                } else {
                    build_outdated_pages(mgr, pages.as_deref())
                }
            }
            BuildPlan::All => build_full_book(mgr),
            BuildPlan::Page(idx) => build_page(mgr, *idx),
            BuildPlan::Range { start, end, flex } => build_page_range(mgr, *start, *end, *flex),
            BuildPlan::Release { .. } => Ok(Vec::new()),
        }
    }

    /// Executes the full build pipeline for this plan.
    ///
    /// Steps: cache refresh → resolve layout → renumber → commit → PDF → BuildResult.
    pub fn run(
        self,
        mut mgr: StateManager,
        skip_pdf: bool,
        skip_cache_update: bool,
    ) -> Result<CommandOutput<BuildResult>> {
        if let BuildPlan::Release { force } = self {
            validate_release(&mgr, force)?;
        }

        // 1. Update image cache
        let (images_processed, dpi_warnings) = refresh_cache(&self, &mut mgr, skip_cache_update)?;

        // 2. Build layout (pure)
        let changed_pages = self.build_layout(&mut mgr)?;

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
        let effective_skip = matches!(self, BuildPlan::Release { .. })
            .then_some(false)
            .unwrap_or(skip_pdf);
        let pdf_path = render_pdf(&ctx, pdf_target(&self), effective_skip)?;

        Ok(CommandOutput {
            result: BuildResult {
                pdf_path,
                pages_rebuilt: changed_pages.clone(),
                images_processed,
                dpi_warnings,
                nothing_to_do: changed_pages.is_empty()
                    && !matches!(self, BuildPlan::Release { .. } | BuildPlan::All),
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
    skip_cache_update: bool,
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

    if skip_cache_update {
        return Ok((0, vec![]));
    }
    let result = update_preview_cache(mgr)?;
    Ok((result.created, vec![]))
}

fn commit_mode(plan: &BuildPlan) -> CommitMode {
    match plan {
        BuildPlan::Auto { .. } => CommitMode::Auto,
        BuildPlan::All
        | BuildPlan::Page(_)
        | BuildPlan::Range { .. }
        | BuildPlan::Release { .. } => CommitMode::Always,
    }
}

fn pdf_target(plan: &BuildPlan) -> PdfTarget {
    if matches!(plan, BuildPlan::Release { .. }) {
        PdfTarget::Final
    } else {
        PdfTarget::Preview
    }
}

fn commit_message(
    plan: &BuildPlan,
    changed_pages: &[usize],
    page_count: usize,
    total_photos: usize,
) -> String {
    match plan {
        BuildPlan::Auto { .. } => {
            if changed_pages.is_empty() {
                "build: no changes".to_string()
            } else {
                format!("build: {} page(s) rebuilt", changed_pages.len())
            }
        }
        BuildPlan::All => format!("rebuild: {} photos redistributed", total_photos),
        BuildPlan::Page(idx) => format!("rebuild: page {}", idx),
        BuildPlan::Range { start, end, .. } => format!("rebuild: pages {}-{}", start, end),
        BuildPlan::Release { .. } => {
            format!("release: {} pages, {} photos", page_count, total_photos)
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── BuildPlan variant construction ────────────────────────────────────────

    #[test]
    fn build_plan_variants_are_constructible() {
        let _ = BuildPlan::Auto { pages: None };
        let _ = BuildPlan::Auto {
            pages: Some(vec![0, 2]),
        };
        let _ = BuildPlan::All;
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
