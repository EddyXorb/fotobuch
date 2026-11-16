//! Typst template compilation to PDF.

use anyhow::{Context, Result};
use lopdf::{Document, Object};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

pub use crate::output::render::{rasterize_page, render_pages_with_world};
pub use crate::output::typst_world::TypstWorld;
/// A rasterized page ready for display (e.g. in egui).
#[derive(Debug)]
pub struct RenderedPage {
    /// 0-based page index within the document.
    pub page: usize,
    pub width: u32,
    pub height: u32,
    /// RGBA pixels, straight (non-premultiplied) alpha.
    pub pixels: Vec<u8>,
}

/// Compiles a Typst template to a `typst_layout::PagedDocument`.
fn compile_to_intermediate_document(template_path: &Path) -> Result<typst_layout::PagedDocument> {
    let mut world = TypstWorld::from_template_path(template_path)?;
    world.reload()?;
    world.compile_document()
}

/// Compiles a Typst template and returns the raw PDF bytes.
fn compile_to_pdf_bytes(template_path: &Path) -> Result<Vec<u8>> {
    let document = compile_to_intermediate_document(template_path)?;
    typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|errors| {
        let error_msg = errors
            .iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<_>>()
            .join("\n");
        anyhow::anyhow!("PDF export failed:\n{}", error_msg)
    })
}

/// Compiles a Typst template to PDF.
///
/// # Arguments
/// * `template_path` - Path to the `.typ` template file
/// * `output_path` - Path where the PDF should be saved
pub fn compile(template_path: &Path, output_path: &Path) -> Result<()> {
    let pdf_bytes = compile_to_pdf_bytes(template_path)?;
    fs::write(output_path, pdf_bytes)
        .with_context(|| format!("Failed to write PDF: {}", output_path.display()))
}

/// Bleed per page of the book, in millimetres.
///
/// The cover (page 0, when active) may have a different bleed than the inner pages.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageBleed {
    /// Bleed of the cover page; `None` when the book has no active cover.
    pub cover_mm: Option<f64>,
    /// Bleed of every inner page.
    pub inner_mm: f64,
}

impl PageBleed {
    /// Same bleed on every page (no separate cover bleed).
    pub fn uniform(bleed_mm: f64) -> Self {
        Self {
            cover_mm: None,
            inner_mm: bleed_mm,
        }
    }

    /// Bleed of the page at the 0-based `page` index.
    pub fn for_page(&self, page: usize) -> f64 {
        match (page, self.cover_mm) {
            (0, Some(cover_mm)) => cover_mm,
            _ => self.inner_mm,
        }
    }

    /// True if at least one page has a positive bleed.
    pub fn any_positive(&self) -> bool {
        self.inner_mm > 0.0 || self.cover_mm.is_some_and(|c| c > 0.0)
    }
}

/// Sets TrimBox and BleedBox in a PDF to mark the printable area vs. bleed zone.
///
/// Each PDF page (MediaBox) must already be sized as `TrimSize + 2 × bleed` of that page.
/// After this call:
/// - `BleedBox` = full page (identical to MediaBox)
/// - `TrimBox`  = page inset by the page's bleed (see [`PageBleed::for_page`]) on every side
// PDF box hierarchy:
//
//  ╔════════════════════════════════════╗← MediaBox (full page, includes bleed, most times same as BleedBox)
//  ║                                    ║
//  ║  ╔══════════════════════════════╗  ║
//  ║  ║                              ║  ║
//  ║  ║  ╔────────────────────────╗  ║← ║  BleedBox (= MediaBox in our case)
//  ║  ║  │                        │  ║  ║
//  ║  ║  │  (Content Area)        │  ║  ║
//  ║  ║  │                        │  ║  ║
//  ║  ║  │     Trimbox            │← ║  ║  TrimBox (actual page size to cut)
//  ║  ║  └────────────────────────┘  ║  ║  Inset from MediaBox by bleed_mm
//  ║  ║     BleedBox                 ║  ║
//  ║  └──────────────────────────────┘  ║
//  ║       MediaBox                     ║
//  └────────────────────────────────────┘
//
fn add_pdf_boxes(pdf_bytes: &[u8], bleed: PageBleed) -> Result<Vec<u8>> {
    const MM_TO_PT: f32 = 72.0 / 25.4;

    let mut doc = Document::load_mem(pdf_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to parse PDF for box annotations: {e}"))?;

    // `get_pages` is a BTreeMap keyed by page number, so values are in page order.
    let page_ids: Vec<_> = doc.get_pages().values().copied().collect();
    for (page_index, page_id) in page_ids.into_iter().enumerate() {
        let bleed_mm = bleed.for_page(page_index);
        let bleed_pt = bleed_mm as f32 * MM_TO_PT;
        let media_box: Vec<Object> = {
            let page = doc
                .get_object(page_id)
                .map_err(|e| anyhow::anyhow!("Page object not found: {e}"))?
                .as_dict()
                .map_err(|e| anyhow::anyhow!("Page is not a dict: {e}"))?;
            page.get(b"MediaBox")
                .map_err(|e| anyhow::anyhow!("MediaBox missing: {e}"))?
                .as_array()
                .map_err(|e| anyhow::anyhow!("MediaBox is not an array: {e}"))?
                .clone()
        };

        let x0 = media_box[0].as_float().unwrap_or(0.0);
        let y0 = media_box[1].as_float().unwrap_or(0.0);
        let x1 = media_box[2]
            .as_float()
            .map_err(|e| anyhow::anyhow!("MediaBox x1 invalid: {e}"))?;
        let y1 = media_box[3]
            .as_float()
            .map_err(|e| anyhow::anyhow!("MediaBox y1 invalid: {e}"))?;

        let trim_box = vec![
            Object::Real(x0 + bleed_pt),
            Object::Real(y0 + bleed_pt),
            Object::Real(x1 - bleed_pt),
            Object::Real(y1 - bleed_pt),
        ];
        let bleed_box = vec![
            Object::Real(x0),
            Object::Real(y0),
            Object::Real(x1),
            Object::Real(y1),
        ];

        let page = doc
            .get_object_mut(page_id)
            .map_err(|e| anyhow::anyhow!("Page object not found: {e}"))?
            .as_dict_mut()
            .map_err(|e| anyhow::anyhow!("Page is not a dict: {e}"))?;
        page.set(b"TrimBox", Object::Array(trim_box));
        page.set(b"BleedBox", Object::Array(bleed_box));

        debug!("Page {page_index}: TrimBox inset by {bleed_mm:.1} mm ({bleed_pt:.2} pt)");
    }

    let mut output = Vec::new();
    doc.save_to(&mut output)
        .map_err(|e| anyhow::anyhow!("Failed to write PDF with boxes: {e}"))?;

    Ok(output)
}

/// Compiles the preview PDF with bleed and sets TrimBox/BleedBox in the output PDF.
/// Template: `{project_root}/{name}.typ` → Output: `{project_root}/{name}.pdf`
pub fn compile_preview(
    project_root: &Path,
    project_name: &str,
    bleed: PageBleed,
) -> Result<PathBuf> {
    let template = project_root.join(format!("{project_name}.typ"));
    let output = project_root.join(format!("{project_name}.pdf"));

    let pdf_bytes = compile_to_pdf_bytes(&template)?;
    let pdf_bytes = if bleed.any_positive() {
        add_pdf_boxes(&pdf_bytes, bleed)?
    } else {
        pdf_bytes
    };
    fs::write(&output, pdf_bytes)
        .with_context(|| format!("Failed to write PDF: {}", output.display()))?;
    Ok(output)
}

/// Compiles the final PDF with bleed and sets TrimBox/BleedBox in the output PDF.
///
/// Generates `final.typ` from `{name}.typ` with `is_final = true`.
/// Template: `{project_root}/final.typ` → Output: `{project_root}/{name}_final.pdf`
pub fn compile_final(project_root: &Path, project_name: &str, bleed: PageBleed) -> Result<PathBuf> {
    let source_template = project_root.join(format!("{project_name}.typ"));
    let final_template = project_root.join("final.typ");
    let output = project_root.join(format!("{}_final.pdf", project_name));

    generate_final_template(&source_template, &final_template)?;

    let pdf_bytes = compile_to_pdf_bytes(&final_template)?;

    let pdf_bytes = if bleed.any_positive() {
        add_pdf_boxes(&pdf_bytes, bleed)?
    } else {
        pdf_bytes
    };

    fs::write(&output, pdf_bytes)
        .with_context(|| format!("Failed to write PDF: {}", output.display()))?;

    Ok(output)
}

/// Generates `final.typ` from the preview template with `is_final = true`.
fn generate_final_template(source: &Path, target: &Path) -> Result<()> {
    let content = fs::read_to_string(source)
        .with_context(|| format!("Failed to read template: {}", source.display()))?;

    let final_content = content.replacen("#let is_final = false", "#let is_final = true", 1);

    fs::write(target, final_content)
        .with_context(|| format!("Failed to write final template: {}", target.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_final_template() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("preview.typ");
        let target = temp.path().join("final.typ");

        fs::write(
            &source,
            "#let is_final = false\n// Preview template\n#text(\"Hello\")",
        )
        .unwrap();

        generate_final_template(&source, &target).unwrap();

        let content = fs::read_to_string(&target).unwrap();
        assert!(content.starts_with("#let is_final = true\n"));
        assert!(
            !content.contains("#let is_final = false"),
            "old declaration must be replaced"
        );
        assert!(content.contains("// Preview template"));
    }

    #[test]
    fn test_compile_simple_template() {
        let temp = TempDir::new().unwrap();
        let template = temp.path().join("test.typ");
        let output = temp.path().join("test.pdf");

        fs::write(&template, "= Hello World\n\nThis is a test.").unwrap();

        let result = compile(&template, &output);

        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());
        assert!(output.exists());
        let metadata = fs::metadata(&output).unwrap();
        assert!(metadata.len() > 0);
    }

    #[test]
    fn test_compile_preview() {
        let temp = TempDir::new().unwrap();
        let template = temp.path().join("mybook.typ");

        fs::write(&template, "= My Book\n\nPreview content.").unwrap();

        let result = compile_preview(temp.path(), "mybook", PageBleed::uniform(0.0));

        assert!(result.is_ok());
        let pdf_path = result.unwrap();
        assert_eq!(pdf_path, temp.path().join("mybook.pdf"));
        assert!(pdf_path.exists());
    }

    #[test]
    fn test_compile_final() {
        let temp = TempDir::new().unwrap();
        let template = temp.path().join("mybook.typ");

        fs::write(
            &template,
            "#let is_final = false\n\n= My Book\n\n#if is_final [Final] else [Preview]",
        )
        .unwrap();

        let result = compile_final(temp.path(), "mybook", PageBleed::uniform(0.0));

        assert!(result.is_ok(), "compile_final failed: {:?}", result.err());
        let pdf_path = result.unwrap();
        assert_eq!(pdf_path, temp.path().join("mybook_final.pdf"));
        assert!(pdf_path.exists());

        let final_typ = temp.path().join("final.typ");
        assert!(final_typ.exists());
        let content = fs::read_to_string(&final_typ).unwrap();
        assert!(content.contains("#let is_final = true"));
        assert!(!content.contains("#let is_final = false"));
    }

    #[test]
    fn test_add_pdf_boxes_sets_trim_and_bleed_box() {
        let temp = TempDir::new().unwrap();
        let template = temp.path().join("bleed_test.typ");
        fs::write(&template, "#set page(width: 210mm, height: 297mm)\nHello").unwrap();
        let pdf_bytes = compile_to_pdf_bytes(&template).unwrap();

        let bleed_mm = 3.0_f64;
        let result = add_pdf_boxes(&pdf_bytes, PageBleed::uniform(bleed_mm));
        assert!(result.is_ok(), "add_pdf_boxes failed: {:?}", result.err());
        let annotated = result.unwrap();

        let doc = Document::load_mem(&annotated).unwrap();
        let page_ids: Vec<_> = doc.get_pages().values().copied().collect();
        assert_eq!(page_ids.len(), 1, "expected exactly one page");

        let page = doc.get_object(page_ids[0]).unwrap().as_dict().unwrap();

        let media_box = page.get(b"MediaBox").unwrap().as_array().unwrap();
        let mx0 = media_box[0].as_float().unwrap();
        let my0 = media_box[1].as_float().unwrap();
        let mx1 = media_box[2].as_float().unwrap();
        let my1 = media_box[3].as_float().unwrap();

        let bleed_box = page.get(b"BleedBox").unwrap().as_array().unwrap();
        assert_eq!(bleed_box[0].as_float().unwrap(), mx0);
        assert_eq!(bleed_box[1].as_float().unwrap(), my0);
        assert_eq!(bleed_box[2].as_float().unwrap(), mx1);
        assert_eq!(bleed_box[3].as_float().unwrap(), my1);

        let bleed_pt = bleed_mm as f32 * (72.0 / 25.4);
        let trim_box = page.get(b"TrimBox").unwrap().as_array().unwrap();
        let tx0 = trim_box[0].as_float().unwrap();
        let ty0 = trim_box[1].as_float().unwrap();
        let tx1 = trim_box[2].as_float().unwrap();
        let ty1 = trim_box[3].as_float().unwrap();

        assert!((tx0 - (mx0 + bleed_pt)).abs() < 0.01);
        assert!((ty0 - (my0 + bleed_pt)).abs() < 0.01);
        assert!((tx1 - (mx1 - bleed_pt)).abs() < 0.01);
        assert!((ty1 - (my1 - bleed_pt)).abs() < 0.01);
    }

    /// Returns `(MediaBox, TrimBox)` of every page as `[x0, y0, x1, y1]` in pt.
    fn media_and_trim_boxes(pdf: &[u8]) -> Vec<([f32; 4], [f32; 4])> {
        let doc = Document::load_mem(pdf).unwrap();
        let read_box = |page: &lopdf::Dictionary, key: &[u8]| -> [f32; 4] {
            let arr = page.get(key).unwrap().as_array().unwrap();
            std::array::from_fn(|i| arr[i].as_float().unwrap())
        };
        doc.get_pages()
            .values()
            .map(|&id| {
                let page = doc.get_object(id).unwrap().as_dict().unwrap();
                (read_box(page, b"MediaBox"), read_box(page, b"TrimBox"))
            })
            .collect()
    }

    fn assert_trim_inset(media: [f32; 4], trim: [f32; 4], bleed_mm: f64) {
        let inset = bleed_mm as f32 * (72.0 / 25.4);
        let expected = [
            media[0] + inset,
            media[1] + inset,
            media[2] - inset,
            media[3] - inset,
        ];
        for (t, e) in trim.iter().zip(expected) {
            assert!(
                (t - e).abs() < 0.01,
                "TrimBox {trim:?} != expected {expected:?}"
            );
        }
    }

    #[test]
    fn test_add_pdf_boxes_uses_cover_bleed_for_first_page() {
        let temp = TempDir::new().unwrap();
        let template = temp.path().join("cover_bleed_test.typ");
        fs::write(
            &template,
            "#set page(width: 420mm, height: 270mm)\nCover\n\
             #page(width: 206mm, height: 256mm)[Inner]",
        )
        .unwrap();
        let pdf_bytes = compile_to_pdf_bytes(&template).unwrap();

        let bleed = PageBleed {
            cover_mm: Some(10.0),
            inner_mm: 3.0,
        };
        let annotated = add_pdf_boxes(&pdf_bytes, bleed).unwrap();

        let boxes = media_and_trim_boxes(&annotated);
        assert_eq!(boxes.len(), 2, "expected cover + one inner page");
        assert_trim_inset(boxes[0].0, boxes[0].1, 10.0);
        assert_trim_inset(boxes[1].0, boxes[1].1, 3.0);
    }

    #[test]
    fn test_page_bleed_for_page() {
        let with_cover = PageBleed {
            cover_mm: Some(10.0),
            inner_mm: 3.0,
        };
        assert_eq!(with_cover.for_page(0), 10.0);
        assert_eq!(with_cover.for_page(1), 3.0);
        assert_eq!(with_cover.for_page(7), 3.0);

        let no_cover = PageBleed::uniform(3.0);
        assert_eq!(no_cover.for_page(0), 3.0);
        assert_eq!(no_cover.for_page(1), 3.0);
    }

    #[test]
    fn test_page_bleed_any_positive() {
        assert!(!PageBleed::uniform(0.0).any_positive());
        assert!(PageBleed::uniform(3.0).any_positive());
        let cover_only = PageBleed {
            cover_mm: Some(5.0),
            inner_mm: 0.0,
        };
        assert!(cover_only.any_positive());
        let zero_cover = PageBleed {
            cover_mm: Some(0.0),
            inner_mm: 0.0,
        };
        assert!(!zero_cover.any_positive());
    }

    #[test]
    fn test_render_pages_correct_dimensions() {
        let temp = TempDir::new().unwrap();
        let mut world = TypstWorld::new(temp.path(), "test").unwrap();
        fs::write(
            temp.path().join("test.typ"),
            "#set page(width: 210mm, height: 297mm)\nHello",
        )
        .unwrap();

        let result = render_pages_with_world(&mut world, &[0], 1.0);
        assert!(result.is_ok(), "render failed: {:?}", result.err());
        let pages = result.unwrap();
        assert_eq!(pages.len(), 1);
        let page = &pages[0];
        assert_eq!(page.page, 0);
        assert!((page.width as i32 - 595).abs() <= 2, "width {}", page.width);
        assert!(
            (page.height as i32 - 842).abs() <= 2,
            "height {}",
            page.height
        );
        assert_eq!(page.pixels.len() as u32, page.width * page.height * 4);
    }

    #[test]
    fn test_render_pages_page_selection() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("multi.typ"),
            "#set page(width: 50mm, height: 50mm)\nPage 1\n#pagebreak()\nPage 2\n#pagebreak()\nPage 3",
        )
        .unwrap();
        let mut world = TypstWorld::new(temp.path(), "multi").unwrap();

        let result = render_pages_with_world(&mut world, &[1], 1.0);
        assert!(result.is_ok(), "{:?}", result.err());
        let pages = result.unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page, 1);
    }

    #[test]
    fn test_render_pages_out_of_range_returns_error() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("single.typ"), "Only one page").unwrap();
        let mut world = TypstWorld::new(temp.path(), "single").unwrap();

        let result = render_pages_with_world(&mut world, &[5], 1.0);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("out of range"), "unexpected error: {msg}");
    }

    #[test]
    fn test_render_pages_straight_alpha_conversion() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("alpha.typ"),
            "#set page(width: 20mm, height: 20mm, fill: white)\n#text(fill: black)[X]",
        )
        .unwrap();
        let mut world = TypstWorld::new(temp.path(), "alpha").unwrap();

        let result = render_pages_with_world(&mut world, &[0], 2.0);
        assert!(result.is_ok(), "{:?}", result.err());
        let page = &result.unwrap()[0];

        let has_opaque = page
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[3] == 255 && p[0] == 255 && p[1] == 255 && p[2] == 255);
        assert!(has_opaque, "expected at least one white opaque pixel");
    }
}
