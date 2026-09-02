use anyhow::Result;
use image::imageops::FilterType;

use crate::output::typst::{RenderedPage, TypstWorld};
use typst::layout::PagedDocument;

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

/// Downsamples `src` so that the longest edge is ≤ `max_edge_px`.
///
/// Aspect ratio is preserved. If `src` is already smaller, a copy with
/// unchanged dimensions is returned.
pub fn downsample(src: &RenderedPage, max_edge_px: u32) -> RenderedPage {
    let (w, h) = (src.width, src.height);
    let max_edge = w.max(h);
    if max_edge <= max_edge_px || max_edge == 0 {
        return RenderedPage {
            page: src.page,
            width: w,
            height: h,
            pixels: src.pixels.clone(),
        };
    }

    let scale = max_edge_px as f64 / max_edge as f64;
    let new_w = ((w as f64 * scale).round() as u32).max(1);
    let new_h = ((h as f64 * scale).round() as u32).max(1);

    let img = image::RgbaImage::from_raw(w, h, src.pixels.clone())
        .expect("pixels must match width*height*4");
    let resized = image::imageops::resize(&img, new_w, new_h, FilterType::Triangle);

    RenderedPage {
        page: src.page,
        width: new_w,
        height: new_h,
        pixels: resized.into_raw(),
    }
}

fn premultiplied_to_straight(pixels: &mut [u8]) {
    for chunk in pixels.as_chunks_mut::<4>().0 {
        let a = chunk[3] as f32 / 255.0;
        if a > 0.0 {
            chunk[0] = (chunk[0] as f32 / a).min(255.0) as u8;
            chunk[1] = (chunk[1] as f32 / a).min(255.0) as u8;
            chunk[2] = (chunk[2] as f32 / a).min(255.0) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_page(w: u32, h: u32) -> RenderedPage {
        RenderedPage {
            page: 0,
            width: w,
            height: h,
            pixels: vec![128u8; (w * h * 4) as usize],
        }
    }

    #[test]
    fn downsample_clamps_to_max_edge() {
        let src = make_page(400, 200);
        let out = downsample(&src, 120);
        assert!(out.width.max(out.height) <= 120);
    }

    #[test]
    fn downsample_keeps_aspect_ratio() {
        let src = make_page(400, 200); // 2:1
        let out = downsample(&src, 120);
        let ratio_src = src.width as f64 / src.height as f64;
        let ratio_out = out.width as f64 / out.height as f64;
        assert!(
            (ratio_src - ratio_out).abs() < 0.05,
            "aspect ratio changed: {ratio_src:.3} vs {ratio_out:.3}"
        );
    }

    #[test]
    fn downsample_noop_when_already_small() {
        let src = make_page(80, 60);
        let out = downsample(&src, 120);
        assert_eq!(out.width, 80);
        assert_eq!(out.height, 60);
    }
}
