mod models;
mod scanner;
mod old_solver;
mod typst_export;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tracing::info;

use models::BookConfig;

/// Photobook solver: distributes photos from timestamped directories
/// across pages and exports to Typst (.typ) and PDF.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Root directory containing timestamped photo subdirectories.
    #[arg(short, long)]
    input: PathBuf,

    /// Output PDF file path.
    #[arg(short, long, default_value = "photobook.pdf")]
    output: PathBuf,

    /// Also write the intermediate .typ source file alongside the PDF.
    #[arg(long, default_value_t = true)]
    write_typ: bool,

    /// Page width in mm.
    #[arg(long, default_value_t = 297.0)]
    page_width: f64,

    /// Page height in mm.
    #[arg(long, default_value_t = 210.0)]
    page_height: f64,

    /// Margin on all sides in mm.
    #[arg(long, default_value_t = 10.0)]
    margin: f64,

    /// Gap between photos in mm.
    #[arg(long, default_value_t = 3.0)]
    gap: f64,

    /// Maximum photos per page.
    #[arg(long, default_value_t = 4)]
    max_photos: usize,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let args = Args::parse();

    let config = BookConfig {
        page_width_mm: args.page_width,
        page_height_mm: args.page_height,
        margin_mm: args.margin,
        gap_mm: args.gap,
        max_photos_per_page: args.max_photos,
    };

    let base_dir = args
        .input
        .canonicalize()
        .unwrap_or_else(|_| args.input.clone());

    // 1. Scan photo directories.
    info!("Scanning {:?} ...", base_dir);
    let groups = scanner::scan_photo_dirs(&base_dir).context("Failed to scan input directory")?;

    let total_photos: usize = groups.iter().map(|g| g.photos.len()).sum();
    info!(
        "Found {} groups with {} photos total",
        groups.len(),
        total_photos
    );

    if total_photos == 0 {
        anyhow::bail!("No supported images found in {:?}", base_dir);
    }

    // 2. Solve layout.
    info!("Solving layout ...");
    let pages = old_solver::solve(&groups, &config);
    info!("Generated {} pages", pages.len());

    // 3. Generate Typst source.
    let typ_source = typst_export::generate_typ(&pages, &config, &base_dir);

    if args.write_typ {
        let stem = args.output.file_stem().unwrap_or("photobook".as_ref());
        let typ_path = base_dir.join(stem).with_extension("typ");
        typst_export::write_typ_file(&typ_source, &typ_path)
            .context("Failed to write .typ file")?;
        info!("Written .typ source to {:?}", typ_path);
    }

    // 4. Compile to PDF.
    info!("Compiling to PDF ...");

    let pdf_bytes =
        typst_export::compile_to_pdf(&typ_source, &base_dir).context("Typst compilation failed")?;

    typst_export::write_pdf(&pdf_bytes, &args.output).context("Failed to write PDF")?;

    info!("Done. PDF written to {:?}", args.output);

    Ok(())
}
