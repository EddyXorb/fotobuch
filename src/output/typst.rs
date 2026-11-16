//! Typst template compilation to PDF.

use anyhow::{Context, Result};
use lopdf::{Document, Object};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::models::{BookConfig, PageSide};

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

/// Bleed of every PDF page, used to place the TrimBox inside the MediaBox.
///
/// Inner pages carry `bleed_inside_mm` on the inside (spine) edge and
/// `bleed_mm` on the other three edges; the side of each page follows
/// [`PageSide`]. The cover (PDF page 0 when `has_cover`) uses `bleed_mm` on
/// all sides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BleedSpec {
    pub bleed_mm: f64,
    pub bleed_inside_mm: f64,
    pub has_cover: bool,
    /// Trim width of inner pages; used to detect templates that do not
    /// render `bleed_inside_mm`.
    pub inner_trim_width_mm: f64,
}

/// Bleed per edge of one page in mm.
#[derive(Debug, Clone, Copy, PartialEq)]
struct EdgeBleed {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl BleedSpec {
    /// Builds the spec from the book configuration.
    pub fn from_book(book: &BookConfig) -> Self {
        Self {
            bleed_mm: book.bleed_mm,
            bleed_inside_mm: book.effective_bleed_inside_mm(),
            has_cover: book.cover.active,
            inner_trim_width_mm: book.page_width_mm,
        }
    }

    fn has_bleed(&self) -> bool {
        self.bleed_mm > 0.0 || self.bleed_inside_mm > 0.0
    }

    fn is_symmetric(&self) -> bool {
        self.bleed_mm == self.bleed_inside_mm
    }

    /// Bleed per edge of the PDF page with 0-based index `pdf_page`.
    /// Pages after the layout (e.g. the appendix) continue the parity.
    fn edges(&self, pdf_page: usize) -> EdgeBleed {
        let b = self.bleed_mm;
        let (left, right) = match PageSide::for_page(pdf_page, self.has_cover) {
            Some(side) => side.to_left_right(self.bleed_inside_mm, b),
            None => (b, b),
        };
        EdgeBleed {
            left,
            right,
            top: b,
            bottom: b,
        }
    }
}

/// Sets TrimBox and BleedBox on every page of a PDF to mark the printable area vs. bleed zone.
///
/// The PDF page (MediaBox) must already be sized as `TrimSize + bleed` per edge (see [`BleedSpec`]).
/// After this call:
/// - `BleedBox` = full page (identical to MediaBox)
/// - `TrimBox`  = page inset by the bleed of each edge
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
//  ║  ║  └────────────────────────┘  ║  ║  Inset from MediaBox by the bleed of each edge
//  ║  ║     BleedBox                 ║  ║  (inner pages: bleed_inside_mm on the spine edge)
//  ║  └──────────────────────────────┘  ║
//  ║       MediaBox                     ║
//  └────────────────────────────────────┘
//
fn add_pdf_boxes(pdf_bytes: &[u8], spec: &BleedSpec) -> Result<Vec<u8>> {
    const MM_TO_PT: f32 = 72.0 / 25.4;
    const WIDTH_TOLERANCE_MM: f64 = 0.5;
    let pt = |mm: f64| mm as f32 * MM_TO_PT;

    let mut doc = Document::load_mem(pdf_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to parse PDF for box annotations: {e}"))?;

    // `get_pages` is keyed by 1-based page number, so values are in document order.
    let page_ids: Vec<_> = doc.get_pages().values().copied().collect();
    for (pdf_page, page_id) in page_ids.into_iter().enumerate() {
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

        let edges = spec.edges(pdf_page);
        let is_cover = spec.has_cover && pdf_page == 0;
        if !spec.is_symmetric() && !is_cover {
            let media_width_mm = f64::from(x1 - x0) / f64::from(MM_TO_PT);
            let expected_mm = spec.inner_trim_width_mm + edges.left + edges.right;
            anyhow::ensure!(
                (media_width_mm - expected_mm).abs() < WIDTH_TOLERANCE_MM,
                "PDF page {pdf_page} is {media_width_mm:.1} mm wide, expected {expected_mm:.1} mm \
                 for bleed_inside_mm = {}. The project template does not render \
                 bleed_inside_mm; update the .typ template from the current fotobuch version.",
                spec.bleed_inside_mm
            );
        }

        // PDF y axis points up: bottom edge is y0, top edge is y1.
        let trim_box = vec![
            Object::Real(x0 + pt(edges.left)),
            Object::Real(y0 + pt(edges.bottom)),
            Object::Real(x1 - pt(edges.right)),
            Object::Real(y1 - pt(edges.top)),
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
    }

    let mut output = Vec::new();
    doc.save_to(&mut output)
        .map_err(|e| anyhow::anyhow!("Failed to write PDF with boxes: {e}"))?;

    debug!(
        "Added TrimBox and BleedBox with bleed {:.1} mm, inside bleed {:.1} mm",
        spec.bleed_mm, spec.bleed_inside_mm
    );

    Ok(output)
}

/// Adds the PDF boxes if the spec has any bleed.
fn with_pdf_boxes(pdf_bytes: Vec<u8>, spec: &BleedSpec) -> Result<Vec<u8>> {
    if spec.has_bleed() {
        add_pdf_boxes(&pdf_bytes, spec)
    } else {
        Ok(pdf_bytes)
    }
}

/// Compiles the preview PDF with bleed and sets TrimBox/BleedBox in the output PDF.
/// Template: `{project_root}/{name}.typ` → Output: `{project_root}/{name}.pdf`
pub fn compile_preview(
    project_root: &Path,
    project_name: &str,
    bleed: &BleedSpec,
) -> Result<PathBuf> {
    let template = project_root.join(format!("{project_name}.typ"));
    let output = project_root.join(format!("{project_name}.pdf"));

    let pdf_bytes = with_pdf_boxes(compile_to_pdf_bytes(&template)?, bleed)?;
    fs::write(&output, pdf_bytes)
        .with_context(|| format!("Failed to write PDF: {}", output.display()))?;
    Ok(output)
}

/// Compiles the final PDF with bleed and sets TrimBox/BleedBox in the output PDF.
///
/// Generates `final.typ` from `{name}.typ` with `is_final = true`.
/// Template: `{project_root}/final.typ` → Output: `{project_root}/{name}_final.pdf`
pub fn compile_final(
    project_root: &Path,
    project_name: &str,
    bleed: &BleedSpec,
) -> Result<PathBuf> {
    let source_template = project_root.join(format!("{project_name}.typ"));
    let final_template = project_root.join("final.typ");
    let output = project_root.join(format!("{}_final.pdf", project_name));

    generate_final_template(&source_template, &final_template)?;

    let pdf_bytes = with_pdf_boxes(compile_to_pdf_bytes(&final_template)?, bleed)?;

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

        let result = compile_preview(temp.path(), "mybook", &BleedSpec::uniform_for_test(0.0));

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

        let result = compile_final(temp.path(), "mybook", &BleedSpec::uniform_for_test(0.0));

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

    impl BleedSpec {
        fn uniform_for_test(bleed_mm: f64) -> Self {
            Self {
                bleed_mm,
                bleed_inside_mm: bleed_mm,
                has_cover: false,
                inner_trim_width_mm: 0.0,
            }
        }
    }

    const PT_PER_MM: f32 = 72.0 / 25.4;

    fn boxes_mm(doc: &Document, page_id: lopdf::ObjectId, key: &[u8]) -> [f32; 4] {
        let page = doc.get_object(page_id).unwrap().as_dict().unwrap();
        let arr = page.get(key).unwrap().as_array().unwrap();
        std::array::from_fn(|i| arr[i].as_float().unwrap() / PT_PER_MM)
    }

    fn assert_box_mm(actual: [f32; 4], expected: [f32; 4]) {
        for (a, e) in actual.iter().zip(expected) {
            assert!((a - e).abs() < 0.01, "box {actual:?} != {expected:?}");
        }
    }

    /// Compiles `pages` pages of the given size (mm) and adds boxes.
    fn facing_pdf(sizes_mm: &[(f64, f64)], spec: &BleedSpec) -> Document {
        let temp = TempDir::new().unwrap();
        let template = temp.path().join("facing.typ");
        let src: String = sizes_mm
            .iter()
            .map(|(w, h)| format!("#page(width: {w}mm, height: {h}mm)[x]\n"))
            .collect();
        fs::write(&template, src).unwrap();
        let pdf_bytes = compile_to_pdf_bytes(&template).unwrap();
        Document::load_mem(&add_pdf_boxes(&pdf_bytes, spec).unwrap()).unwrap()
    }

    fn cewe_spec(has_cover: bool) -> BleedSpec {
        BleedSpec {
            bleed_mm: 3.0,
            bleed_inside_mm: 0.0,
            has_cover,
            inner_trim_width_mm: 290.0,
        }
    }

    #[test]
    fn inside_bleed_boxes_pages_1_2_3_without_cover() {
        let doc = facing_pdf(&[(293.0, 296.0); 3], &cewe_spec(false));
        let ids: Vec<_> = doc.get_pages().values().copied().collect();
        // Page 1 (right-hand): spine left → no bleed on the left.
        // Page 2 (left-hand): spine right → no bleed on the right.
        let expected_trim = [
            [0.0, 3.0, 290.0, 293.0],
            [3.0, 3.0, 293.0, 293.0],
            [0.0, 3.0, 290.0, 293.0],
        ];
        for (id, trim) in ids.iter().zip(expected_trim) {
            assert_box_mm(boxes_mm(&doc, *id, b"MediaBox"), [0.0, 0.0, 293.0, 296.0]);
            assert_box_mm(boxes_mm(&doc, *id, b"BleedBox"), [0.0, 0.0, 293.0, 296.0]);
            assert_box_mm(boxes_mm(&doc, *id, b"TrimBox"), trim);
        }
    }

    #[test]
    fn inside_bleed_boxes_with_cover_keep_cover_uniform() {
        let doc = facing_pdf(
            &[
                (500.0, 306.0),
                (293.0, 296.0),
                (293.0, 296.0),
                (293.0, 296.0),
            ],
            &cewe_spec(true),
        );
        let ids: Vec<_> = doc.get_pages().values().copied().collect();
        assert_box_mm(boxes_mm(&doc, ids[0], b"TrimBox"), [3.0, 3.0, 497.0, 303.0]);
        assert_box_mm(boxes_mm(&doc, ids[1], b"TrimBox"), [0.0, 3.0, 290.0, 293.0]);
        assert_box_mm(boxes_mm(&doc, ids[2], b"TrimBox"), [3.0, 3.0, 293.0, 293.0]);
        assert_box_mm(boxes_mm(&doc, ids[3], b"TrimBox"), [0.0, 3.0, 290.0, 293.0]);
    }

    #[test]
    fn inside_bleed_rejects_template_without_inside_bleed() {
        let temp = TempDir::new().unwrap();
        let template = temp.path().join("old.typ");
        fs::write(&template, "#set page(width: 296mm, height: 296mm)\nx").unwrap();
        let pdf_bytes = compile_to_pdf_bytes(&template).unwrap();
        let err = add_pdf_boxes(&pdf_bytes, &cewe_spec(false)).unwrap_err();
        assert!(err.to_string().contains("bleed_inside_mm"));
    }

    #[test]
    fn test_add_pdf_boxes_sets_trim_and_bleed_box() {
        let temp = TempDir::new().unwrap();
        let template = temp.path().join("bleed_test.typ");
        fs::write(&template, "#set page(width: 210mm, height: 297mm)\nHello").unwrap();
        let pdf_bytes = compile_to_pdf_bytes(&template).unwrap();

        let bleed_mm = 3.0_f64;
        let result = add_pdf_boxes(&pdf_bytes, &BleedSpec::uniform_for_test(bleed_mm));
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
