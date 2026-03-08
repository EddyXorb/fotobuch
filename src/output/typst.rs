//! Typst template compilation to PDF

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::{FontSlot, Fonts};

/// Compiles a Typst template to PDF.
/// Uses the typst crate directly (no external binary needed).
///
/// # Arguments
/// * `template_path` - Path to the `.typ` template file
/// * `output_path` - Path where the PDF should be saved
pub fn compile(template_path: &Path, output_path: &Path) -> Result<()> {
    // Read template content
    let content = fs::read_to_string(template_path)
        .with_context(|| format!("Failed to read template: {}", template_path.display()))?;

    // Create world
    let world = SimpleWorld::new(template_path, content)?;

    // Compile
    let result = typst::compile(&world);

    // Check for warnings and errors
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

    // Export to PDF
    let pdf_bytes =
        typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|errors| {
            let error_msg = errors
                .iter()
                .map(|e| format!("{:?}", e))
                .collect::<Vec<_>>()
                .join("\n");
            anyhow::anyhow!("PDF export failed:\n{}", error_msg)
        })?;

    // Write PDF
    fs::write(output_path, pdf_bytes)
        .with_context(|| format!("Failed to write PDF: {}", output_path.display()))?;

    Ok(())
}

/// Compiles the preview PDF.
/// Template: `{project_root}/{name}.typ` → Output: `{project_root}/{name}.pdf`
pub fn compile_preview(project_root: &Path, project_name: &str) -> Result<PathBuf> {
    let template = project_root.join(format!("{project_name}.typ"));
    let output = project_root.join(format!("{project_name}.pdf"));
    compile(&template, &output)?;
    Ok(output)
}

/// Compiles the final PDF.
/// Generates `final.typ` from `{name}.typ` with `is_final = true`.
/// Template: `{project_root}/final.typ` → Output: `{project_root}/final.pdf`
pub fn compile_final(project_root: &Path, project_name: &str) -> Result<PathBuf> {
    let source_template = project_root.join(format!("{project_name}.typ"));
    let final_template = project_root.join("final.typ");
    let output = project_root.join("final.pdf");

    // Generate final.typ with is_final = true
    generate_final_template(&source_template, &final_template)?;

    // Compile
    compile(&final_template, &output)?;

    Ok(output)
}

/// Generates `final.typ` from the preview template with `is_final = true`.
fn generate_final_template(source: &Path, target: &Path) -> Result<()> {
    let content = fs::read_to_string(source)
        .with_context(|| format!("Failed to read template: {}", source.display()))?;

    // Prepend is_final = true before the template content
    let final_content = format!("#let is_final = true\n\n{}", content);

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

        // Create file ID for main source
        let main_id = FileId::new(None, VirtualPath::new(path));

        Ok(Self {
            main_id,
            main_source: Source::new(main_id, content),
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
            Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
        }
    }

    fn file(&self, _id: FileId) -> FileResult<Bytes> {
        Err(FileError::NotFound(PathBuf::from(
            "file access not implemented",
        )))
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

        fs::write(&source, "// Preview template\n#text(\"Hello\")").unwrap();

        generate_final_template(&source, &target).unwrap();

        let content = fs::read_to_string(&target).unwrap();
        assert!(content.starts_with("#let is_final = true\n\n"));
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

        let result = compile_preview(temp.path(), "mybook");

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
            "= My Book\n\n#if is_final [Final] else [Preview]",
        )
        .unwrap();

        let result = compile_final(temp.path(), "mybook");

        assert!(result.is_ok());
        let pdf_path = result.unwrap();
        assert_eq!(pdf_path, temp.path().join("final.pdf"));
        assert!(pdf_path.exists());

        // Verify final.typ was created with is_final = true
        let final_typ = temp.path().join("final.typ");
        assert!(final_typ.exists());
        let content = fs::read_to_string(&final_typ).unwrap();
        assert!(content.starts_with("#let is_final = true\n\n"));
    }
}
