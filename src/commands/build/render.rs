use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

use crate::output::typst;
use crate::state_manager::StateManager;

/// Data captured from `StateManager` before it is consumed by `finish`/`finish_always`.
pub struct RenderContext {
    pub project_root: PathBuf,
    pub project_name: String,
    pub bleed_mm: f64,
}

impl RenderContext {
    pub fn capture(mgr: &StateManager) -> Self {
        Self {
            project_root: mgr.project_root().to_owned(),
            project_name: mgr.project_name().to_string(),
            bleed_mm: mgr.state.config.book.bleed_mm,
        }
    }
}

pub enum PdfTarget {
    Preview,
    Final,
}

/// Renders the PDF or returns only the output path when `skip_pdf` is true.
pub fn render_pdf(ctx: &RenderContext, target: PdfTarget, skip_pdf: bool) -> Result<PathBuf> {
    let root = &ctx.project_root;
    let name = &ctx.project_name;
    let bleed = ctx.bleed_mm;
    if skip_pdf {
        return Ok(root.join(format!("{name}.pdf")));
    }
    match target {
        PdfTarget::Preview => {
            let path = typst::compile_preview(root, name, bleed)?;
            info!("PDF updated: {}", path.display());
            Ok(path)
        }
        PdfTarget::Final => {
            let path = typst::compile_final(root, name, bleed)?;
            info!("Final PDF generated: {}", path.display());
            Ok(path)
        }
    }
}
