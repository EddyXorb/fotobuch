use anyhow::Result;

use typst::layout::PagedDocument;
use crate::output::typst::{RenderedPage, TypstWorld};

/// Rasterises a single page from an already-compiled document.
/// Stateless and parallelisable — the caller decides the scheduling.
pub fn rasterize_page(doc: &PagedDocument, page: usize, pixel_per_pt: f32) -> Result<RenderedPage> {
    let p = doc.pages.get(page).ok_or_else(|| {
        anyhow::anyhow!(
            "page index {page} out of range (document has {} page(s))",
            doc.pages.len()
        )
    })?;

    let pixmap = typst_render::render(p, pixel_per_pt);
    let width = pixmap.width();
    let height = pixmap.height();
    let mut pixels = pixmap.take();
    premultiplied_to_straight(&mut pixels);

    Ok(RenderedPage {
        page,
        width,
        height,
        pixels,
    })
}

/// Convenience: reload → compile → rasterise the requested pages.
pub fn render_pages_with_world(
    world: &mut TypstWorld,
    pages: &[usize],
    pixel_per_pt: f32,
) -> Result<Vec<RenderedPage>> {
    world.reload()?;
    let doc = world.compile_document()?;
    pages
        .iter()
        .map(|&p| rasterize_page(&doc, p, pixel_per_pt))
        .collect()
}

fn premultiplied_to_straight(pixels: &mut [u8]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let a = chunk[3] as f32 / 255.0;
        if a > 0.0 {
            chunk[0] = (chunk[0] as f32 / a).min(255.0) as u8;
            chunk[1] = (chunk[1] as f32 / a).min(255.0) as u8;
            chunk[2] = (chunk[2] as f32 / a).min(255.0) as u8;
        }
    }
}
