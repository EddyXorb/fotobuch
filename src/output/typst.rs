//! Typst template compilation to PDF

use anyhow::{Context, Result};
use lopdf::{Document, Object};
use std::fs;
use std::path::{Path, PathBuf};
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::{FontSlot, Fonts};

/// Compiles a Typst template and returns the raw PDF bytes.
fn compile_to_bytes(template_path: &Path) -> Result<Vec<u8>> {
    let content = fs::read_to_string(template_path)
        .with_context(|| format!("Failed to read template: {}", template_path.display()))?;

    let world = SimpleWorld::new(template_path, content)?;

    let result = typst::compile(&world);

    if !result.warnings.is_empty() {
        eprintln!("Typst warnings:");
        for warning in &result.warnings {
            eprintln!("  {:?}", warning);
        }
    }

    let document = result.output.map_err(|errors| {
        let error_msg = errors
            .iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<_>>()
            .join("\n");
        anyhow::anyhow!("Typst compilation failed:\n{}", error_msg)
    })?;

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
/// Uses the typst crate directly (no external binary needed).
///
/// # Arguments
/// * `template_path` - Path to the `.typ` template file
/// * `output_path` - Path where the PDF should be saved
pub fn compile(template_path: &Path, output_path: &Path) -> Result<()> {
    let pdf_bytes = compile_to_bytes(template_path)?;
    fs::write(output_path, pdf_bytes)
        .with_context(|| format!("Failed to write PDF: {}", output_path.display()))
}

/// Sets TrimBox and BleedBox in a PDF to mark the printable area vs. bleed zone.
///
/// The PDF page (MediaBox) must already be sized as `TrimSize + 2 × bleed_mm` on each side.
/// After this call:
/// - `BleedBox` = full page (identical to MediaBox)
/// - `TrimBox`  = page inset by `bleed_mm` on every side
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
fn add_pdf_boxes(pdf_bytes: &[u8], bleed_mm: f64) -> Result<Vec<u8>> {
    const MM_TO_PT: f32 = 72.0 / 25.4;
    let bleed_pt = bleed_mm as f32 * MM_TO_PT;

    let mut doc = Document::load_mem(pdf_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to parse PDF for box annotations: {e}"))?;

    let page_ids: Vec<_> = doc.get_pages().values().copied().collect();
    for page_id in page_ids {
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
    }

    let mut output = Vec::new();
    doc.save_to(&mut output)
        .map_err(|e| anyhow::anyhow!("Failed to write PDF with boxes: {e}"))?;
    Ok(output)
}

/// Compiles the preview PDF with bleed and sets TrimBox/BleedBox in the output PDF.
/// Template: `{project_root}/{name}.typ` → Output: `{project_root}/{name}.pdf`
/// We *could* generate from string only, but reading it from the file makes it easier to debug and avoids surprises.
/// This way we know for sure that we use only what was committed by the state manager.
pub fn compile_preview(project_root: &Path, project_name: &str, bleed_mm: f64) -> Result<PathBuf> {
    let template = project_root.join(format!("{project_name}.typ"));
    let output = project_root.join(format!("{project_name}.pdf"));

    let pdf_bytes = compile_to_bytes(&template)?;
    let pdf_bytes = if bleed_mm > 0.0 {
        add_pdf_boxes(&pdf_bytes, bleed_mm)?
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
///
/// When `bleed_mm > 0`, the PDF page size is `TrimSize + 2×bleed_mm` and the
/// PDF boxes are set accordingly so professional print shops can read the
/// correct trim and bleed areas.
///
/// The **reason** why the template is named "final.typ" is that the user should not edit it directly, but instead work on the {name}.typ
/// template, otherwise there might be multiple sources of truth, which can lead to confusion.
/// At the other hand the user should be able to inspect the final.typ to see how the final PDF is generated,
/// and it can also be useful for debugging to have the final template available as a separate file,
/// so we write it to disk instead of keeping it in memory.
pub fn compile_final(project_root: &Path, project_name: &str, bleed_mm: f64) -> Result<PathBuf> {
    let source_template = project_root.join(format!("{project_name}.typ"));
    let final_template = project_root.join("final.typ");
    let output = project_root.join(format!("{}_final.pdf", project_name));

    generate_final_template(&source_template, &final_template)?;

    let pdf_bytes = compile_to_bytes(&final_template)?;

    let pdf_bytes = if bleed_mm > 0.0 {
        add_pdf_boxes(&pdf_bytes, bleed_mm)?
    } else {
        pdf_bytes
    };

    fs::write(&output, pdf_bytes)
        .with_context(|| format!("Failed to write PDF: {}", output.display()))?;

    Ok(output)
}

/// Generates `final.typ` from the preview template with `is_final = true`.
///
/// Replaces `#let is_final = false` in the template instead of prepending a second
/// binding, since a later Typst `let` binding would shadow an earlier one.
fn generate_final_template(source: &Path, target: &Path) -> Result<()> {
    let content = fs::read_to_string(source)
        .with_context(|| format!("Failed to read template: {}", source.display()))?;

    let final_content = content.replacen("#let is_final = false", "#let is_final = true", 1);

    fs::write(target, final_content)
        .with_context(|| format!("Failed to write final template: {}", target.display()))?;

    Ok(())
}

/// Minimal Typst World implementation
struct SimpleWorld {
    /// The main template file ID
    main_id: FileId,
    /// The main template source
    main_source: Source,
    /// Root directory for resolving relative paths
    root: PathBuf,
    /// Font book
    book: LazyHash<FontBook>,
    /// Fonts
    fonts: Vec<FontSlot>,
    /// Standard library
    library: LazyHash<Library>,
}

impl SimpleWorld {
    fn new(path: &Path, content: String) -> Result<Self> {
        // Load system fonts
        let fonts = Fonts::searcher().search();

        // Get root directory (parent of template file)
        let root = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Template path has no parent"))?
            .to_path_buf();

        // Create file ID relative to root so that as_rootless_path() returns
        // just the filename, enabling correct path resolution in source()/file().
        let vpath = VirtualPath::within_root(path, &root)
            .ok_or_else(|| anyhow::anyhow!("Template path is not within root directory"))?;
        let main_id = FileId::new(None, vpath);

        Ok(Self {
            main_id,
            main_source: Source::new(main_id, content),
            root,
            book: LazyHash::new(fonts.book),
            fonts: fonts.fonts,
            library: LazyHash::new(Library::default()),
        })
    }
}

impl World for SimpleWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_id {
            Ok(self.main_source.clone())
        } else {
            // Try to load file from filesystem
            let relative_path = id.vpath().as_rootless_path();
            let full_path = self.root.join(relative_path);

            fs::read_to_string(&full_path)
                .map(|content| Source::new(id, content))
                .map_err(|_| FileError::NotFound(relative_path.into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let relative_path = id.vpath().as_rootless_path();
        let full_path = self.root.join(relative_path);

        fs::read(&full_path)
            .map(Bytes::new)
            .map_err(|_| FileError::NotFound(relative_path.into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).and_then(|slot| slot.get())
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        Datetime::from_ymd(2026, 1, 1)
    }
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

        // Create a minimal Typst template
        fs::write(&template, "= Hello World\n\nThis is a test.").unwrap();

        // Compile
        let result = compile(&template, &output);

        // Should succeed
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

        // PDF should exist and have content
        assert!(output.exists());
        let metadata = fs::metadata(&output).unwrap();
        assert!(metadata.len() > 0);
    }

    #[test]
    fn test_compile_preview() {
        let temp = TempDir::new().unwrap();
        let template = temp.path().join("mybook.typ");

        fs::write(&template, "= My Book\n\nPreview content.").unwrap();

        let result = compile_preview(temp.path(), "mybook", 0.0);

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

        let result = compile_final(temp.path(), "mybook", 0.0);

        assert!(result.is_ok(), "compile_final failed: {:?}", result.err());
        let pdf_path = result.unwrap();
        assert_eq!(pdf_path, temp.path().join("mybook_final.pdf"));
        assert!(pdf_path.exists());

        // Verify final.typ was created with is_final = true (declaration replaced)
        let final_typ = temp.path().join("final.typ");
        assert!(final_typ.exists());
        let content = fs::read_to_string(&final_typ).unwrap();
        assert!(content.contains("#let is_final = true"));
        assert!(!content.contains("#let is_final = false"));
    }

    #[test]
    fn test_add_pdf_boxes_sets_trim_and_bleed_box() {
        // Compile a minimal PDF with a known page size (A4: 595.28 × 841.89 pt)
        let temp = TempDir::new().unwrap();
        let template = temp.path().join("bleed_test.typ");
        fs::write(&template, "#set page(width: 210mm, height: 297mm)\nHello").unwrap();
        let pdf_bytes = compile_to_bytes(&template).unwrap();

        let bleed_mm = 3.0_f64;
        let result = add_pdf_boxes(&pdf_bytes, bleed_mm);
        assert!(result.is_ok(), "add_pdf_boxes failed: {:?}", result.err());
        let annotated = result.unwrap();

        // Parse the annotated PDF and check the boxes on the first page
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
        assert_eq!(
            bleed_box[0].as_float().unwrap(),
            mx0,
            "BleedBox x0 must equal MediaBox x0"
        );
        assert_eq!(
            bleed_box[1].as_float().unwrap(),
            my0,
            "BleedBox y0 must equal MediaBox y0"
        );
        assert_eq!(
            bleed_box[2].as_float().unwrap(),
            mx1,
            "BleedBox x1 must equal MediaBox x1"
        );
        assert_eq!(
            bleed_box[3].as_float().unwrap(),
            my1,
            "BleedBox y1 must equal MediaBox y1"
        );

        let bleed_pt = bleed_mm as f32 * (72.0 / 25.4);
        let trim_box = page.get(b"TrimBox").unwrap().as_array().unwrap();
        let tx0 = trim_box[0].as_float().unwrap();
        let ty0 = trim_box[1].as_float().unwrap();
        let tx1 = trim_box[2].as_float().unwrap();
        let ty1 = trim_box[3].as_float().unwrap();

        assert!(
            (tx0 - (mx0 + bleed_pt)).abs() < 0.01,
            "TrimBox x0 off: {tx0} vs {}",
            mx0 + bleed_pt
        );
        assert!(
            (ty0 - (my0 + bleed_pt)).abs() < 0.01,
            "TrimBox y0 off: {ty0} vs {}",
            my0 + bleed_pt
        );
        assert!(
            (tx1 - (mx1 - bleed_pt)).abs() < 0.01,
            "TrimBox x1 off: {tx1} vs {}",
            mx1 - bleed_pt
        );
        assert!(
            (ty1 - (my1 - bleed_pt)).abs() < 0.01,
            "TrimBox y1 off: {ty1} vs {}",
            my1 - bleed_pt
        );
    }
}
