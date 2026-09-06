use super::BuildResult;
use super::build_layout::{build_full_book, build_outdated_pages, build_page, build_page_range};
use super::cache::{CacheRefresh, refresh_final_cache, refresh_preview_cache};
use super::errors::BuildError;
use super::render::{PdfTarget, RenderContext, render_pdf};
use crate::commands::CommandOutput;
use crate::state_manager::StateManager;
use anyhow::Result;

pub enum CommitMode {
    Auto,
    Always,
}

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
                if mgr.state().layout.is_empty() {
                    build_full_book(&mut mgr.get_write_layout_state())
                } else {
                    let mut outdated = mgr.outdated_pages_indices();
                    if let Some(filter) = pages.as_deref() {
                        outdated.retain(|p| filter.contains(p));
                    }
                    build_outdated_pages(&mut mgr.get_write_layout_state(), &outdated)
                }
            }
            BuildPlan::All => build_full_book(&mut mgr.get_write_layout_state()),
            BuildPlan::Page(idx) => build_page(&mut mgr.get_write_layout_state(), *idx),
            BuildPlan::Range { start, end, flex } => {
                build_page_range(&mut mgr.get_write_layout_state(), *start, *end, *flex)
            }
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
        let cache = refresh_cache(&self, &mut mgr, skip_cache_update)?;

        // 2. Build layout (pure)
        let changed_pages = self.build_layout(&mut mgr)?;

        let ctx = RenderContext::capture(&mgr);
        let page_count = mgr.state().layout.len();
        let total_photos: usize = mgr.state().layout.iter().map(|p| p.photos.len()).sum();

        // 4. Commit
        let msg = commit_message(&self, &changed_pages, page_count, total_photos);
        let changed_state = match commit_mode(&self) {
            CommitMode::Always => Some(mgr.finish_always(&msg)?),
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
                images_processed: cache.images_processed,
                dpi_warnings: cache.dpi_warnings,
                nothing_to_do: changed_pages.is_empty()
                    && !matches!(self, BuildPlan::Release { .. } | BuildPlan::All),
            },
            changed_state,
        })
    }
}

// ── pipeline helpers ─────────────────────────────────────────────────────────

fn validate_release(mgr: &StateManager, force: bool) -> Result<()> {
    if mgr.state().layout.is_empty() {
        return Err(BuildError::NoLayout.into());
    }
    if !force {
        let pages: Vec<_> = mgr
            .outdated_pages_indices()
            .into_iter()
            .filter(|i| !mgr.state().config.book.cover.active || *i != 0)
            .collect();
        if !pages.is_empty() {
            return Err(BuildError::LayoutDirty { pages }.into());
        }
    }
    Ok(())
}

fn refresh_cache(
    plan: &BuildPlan,
    mgr: &mut StateManager,
    skip_cache_update: bool,
) -> Result<CacheRefresh> {
    if let BuildPlan::Release { .. } = plan {
        return refresh_final_cache(mgr);
    }
    if skip_cache_update {
        return Ok(CacheRefresh::images_only(0));
    }
    let result = refresh_preview_cache(mgr)?;
    Ok(CacheRefresh::images_only(result.created))
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
