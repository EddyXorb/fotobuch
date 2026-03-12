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
        let x1 = media_box[2].as_float()
            .map_err(|e| anyhow::anyhow!("MediaBox x1 invalid: {e}"))?;
        let y1 = media_box[3].as_float()
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
/// Template: `{project_root}/final.typ` → Output: `{project_root}/final.pdf`
///
/// When `bleed_mm > 0`, the PDF page size is `TrimSize + 2×bleed_mm` and the
/// PDF boxes are set accordingly so professional print shops can read the
/// correct trim and bleed areas.
pub fn compile_final(project_root: &Path, project_name: &str, bleed_mm: f64) -> Result<PathBuf> {
    let source_template = project_root.join(format!("{project_name}.typ"));
    let final_template = project_root.join("final.typ");
    let output = project_root.join("final.pdf");

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
        Datetime::from_ymd(2024, 1, 1)
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

        fs::write(&source, "#let is_final = false\n// Preview template\n#text(\"Hello\")").unwrap();

        generate_final_template(&source, &target).unwrap();

        let content = fs::read_to_string(&target).unwrap();
        assert!(content.starts_with("#let is_final = true\n"));
        assert!(!content.contains("#let is_final = false"), "old declaration must be replaced");
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
        assert_eq!(pdf_path, temp.path().join("final.pdf"));
        assert!(pdf_path.exists());

        // Verify final.typ was created with is_final = true (declaration replaced)
        let final_typ = temp.path().join("final.typ");
        assert!(final_typ.exists());
        let content = fs::read_to_string(&final_typ).unwrap();
        assert!(content.contains("#let is_final = true"));
        assert!(!content.contains("#let is_final = false"));
    }
}
