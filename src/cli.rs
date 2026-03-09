//! Command-line interface for the photobook solver.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use photobook_solver::commands::{self};
use std::path::PathBuf;

/// Photobook layout solver and project manager
#[derive(Parser, Debug)]
#[command(version, about = "Photobook layout solver and project manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Trait for executing commands
pub trait Execute {
    /// Execute the command and return a result
    fn execute(&self) -> Result<()>;
}

/// Available subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add photos to the project
    Add {
        /// Directories or files containing photos to add
        paths: Vec<PathBuf>,

        /// Allow adding duplicate photos (by hash)
        #[arg(long)]
        allow_duplicates: bool,
    },

    /// Calculate layout and generate preview or final PDF
    Build {
        /// Generate final high-quality PDF at 300 DPI (requires clean state)
        #[arg(long)]
        release: bool,

        /// Only rebuild specific pages (1-based, comma-separated or repeated flag)
        #[arg(long, value_delimiter = ',')]
        pages: Option<Vec<usize>>,
    },

    /// Force re-optimization of pages or page ranges
    Rebuild {
        /// Single page to rebuild (1-based)
        #[arg(long, conflicts_with_all = ["range_start", "all"])]
        page: Option<usize>,

        /// Start of page range (1-based, requires --range-end)
        #[arg(long, requires = "range_end", conflicts_with_all = ["page", "all"])]
        range_start: Option<usize>,

        /// End of page range (1-based, requires --range-start)
        #[arg(long, requires = "range_start", conflicts_with_all = ["page", "all"])]
        range_end: Option<usize>,

        /// Allow page count to vary by +/- N (only with range)
        #[arg(long, default_value = "0", requires = "range_start")]
        flex: usize,

        /// Rebuild all pages from scratch
        #[arg(long, conflicts_with_all = ["page", "range_start", "range_end"])]
        all: bool,
    },

    /// Place unplaced photos into the book
    Place {
        /// Only place photos matching this regex pattern
        #[arg(long)]
        filter: Option<String>,

        /// Place all matching photos onto this specific page (1-based)
        #[arg(long)]
        into: Option<usize>,
    },

    /// Remove photos or groups from the book
    Remove {
        /// Photos, group names, or regex patterns to remove (can be repeated)
        patterns: Vec<String>,

        /// Only remove from layout, keep photos in the project (makes them unplaced)
        #[arg(long)]
        keep_files: bool,
    },

    /// Project management commands
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },
}

/// Project subcommands
#[derive(Subcommand, Debug)]
pub enum ProjectCommands {
    /// Create a new photobook project
    New {
        /// Project name (used as branch name fotobuch/<name>)
        name: String,

        /// Page width in millimeters
        #[arg(long)]
        width: f64,

        /// Page height in millimeters
        #[arg(long)]
        height: f64,

        /// Bleed margin in millimeters
        #[arg(long, default_value = "3")]
        bleed: f64,

        /// Parent directory where project will be created (default: current directory)
        #[arg(long)]
        parent_dir: Option<PathBuf>,

        /// Suppress welcome message
        #[arg(long, default_value_t = false)]
        quiet: bool,
    },
}

impl Execute for Commands {
    fn execute(&self) -> Result<()> {
        match self {
            Commands::Add {
                paths,
                allow_duplicates,
            } => {
                let project_root = std::env::current_dir()
                    .context("Failed to determine current directory")?;

                let config = commands::AddConfig {
                    paths: paths.clone(),
                    allow_duplicates: *allow_duplicates,
                };

                let result = commands::add(&project_root, &config)?;

                if result.groups_added.is_empty() {
                    println!("ℹ️  No new photos added (all skipped).");
                } else {
                    println!("✅ Added {} photos in {} groups", 
                        result.groups_added.iter().map(|g| g.photo_count).sum::<usize>(),
                        result.groups_added.len()
                    );
                    
                    for group in &result.groups_added {
                        println!("   📁 {} — {} photos ({})", 
                            group.name, 
                            group.photo_count, 
                            group.timestamp
                        );
                    }
                }

                if result.skipped > 0 {
                    println!("⏭️  Skipped {} duplicate photos", result.skipped);
                }

                if !result.warnings.is_empty() {
                    println!("⚠️  Warnings:");
                    for warning in &result.warnings {
                        println!("   - {}", warning);
                    }
                }

                Ok(())
            }
            Commands::Build { release, pages } => {
                let project_root = std::env::current_dir()
                    .context("Failed to determine current directory")?;

                let config = commands::build::BuildConfig {
                    release: *release,
                    pages: pages.clone(),
                };

                let result = commands::build::build(&project_root, &config)?;

                if result.nothing_to_do {
                    println!("Nothing to do.");
                    return Ok(());
                }

                if !result.pages_rebuilt.is_empty() {
                    println!("Rebuilt {} page(s): {:?}", result.pages_rebuilt.len(), result.pages_rebuilt);
                }
                println!("PDF: {}", result.pdf_path.display());

                if !result.dpi_warnings.is_empty() {
                    println!("\nWARNING: {} photo(s) below 300 DPI:", result.dpi_warnings.len());
                    for w in &result.dpi_warnings {
                        println!("  Page {}: {} — {:.0} DPI", w.page, w.photo_id, w.actual_dpi);
                    }
                }

                Ok(())
            }
            Commands::Rebuild {
                page,
                range_start,
                range_end,
                flex,
                all,
            } => {
                let project_root = std::env::current_dir()
                    .context("Failed to determine current directory")?;

                // Determine scope
                let scope = if *all {
                    commands::rebuild::RebuildScope::All
                } else if let Some(p) = page {
                    commands::rebuild::RebuildScope::SinglePage(*p)
                } else if let (Some(start), Some(end)) = (range_start, range_end) {
                    commands::rebuild::RebuildScope::Range {
                        start: *start,
                        end: *end,
                        flex: *flex,
                    }
                } else {
                    // Default to all if no specific scope given
                    commands::rebuild::RebuildScope::All
                };

                let result = commands::rebuild::rebuild(&project_root, scope)?;

                if !result.pages_rebuilt.is_empty() {
                    println!("✅ Rebuilt {} page(s): {:?}",
                        result.pages_rebuilt.len(),
                        result.pages_rebuilt
                    );
                }
                println!("📄 PDF: {}", result.pdf_path.display());

                Ok(())
            }
            Commands::Place { filter, into } => {
                let project_root = std::env::current_dir()
                    .context("Failed to determine current directory")?;

                let config = commands::place::PlaceConfig {
                    filter: filter.clone(),
                    into_page: *into,
                };

                let result = commands::place::place(&project_root, &config)?;

                if result.photos_placed == 0 {
                    println!("ℹ️  No photos to place.");
                } else {
                    let pages_str = if result.pages_affected.len() == 1 {
                        format!("page {}", result.pages_affected[0])
                    } else {
                        format!("pages {:?}", result.pages_affected)
                    };
                    println!("✅ Placed {} photo(s) onto {}", result.photos_placed, pages_str);
                    println!("🔄 Run 'fotobuch build' or 'fotobuch rebuild' to regenerate PDFs.");
                }

                Ok(())
            }
            Commands::Remove { patterns, keep_files } => {
                let project_root = std::env::current_dir()
                    .context("Failed to determine current directory")?;

                let config = commands::remove::RemoveConfig {
                    patterns: patterns.clone(),
                    keep_files: *keep_files,
                };

                let result = commands::remove::remove(&project_root, &config)?;

                if result.photos_removed == 0 && result.placements_removed == 0 {
                    println!("ℹ️  No photos matched the pattern(s).");
                } else {
                    if result.photos_removed > 0 {
                        println!("✅ Removed {} photo(s) from project", result.photos_removed);
                        if !result.groups_removed.is_empty() {
                            println!("   Removed groups: {}", result.groups_removed.join(", "));
                        }
                    }
                    if result.placements_removed > 0 {
                        let pages_str = if result.pages_affected.len() == 1 {
                            format!("page {}", result.pages_affected[0])
                        } else {
                            format!("pages {:?}", result.pages_affected)
                        };
                        println!("✅ Removed {} placement(s) from {}", result.placements_removed, pages_str);
                    }
                    if *keep_files {
                        println!("ℹ️  Photos kept in project as unplaced.");
                    }
                    println!("🔄 Run 'fotobuch build' or 'fotobuch rebuild' to regenerate PDFs.");
                }

                Ok(())
            }
            Commands::Project { command } => command.execute(),
        }
    }
}

impl Execute for ProjectCommands {
    fn execute(&self) -> Result<()> {
        match self {
            ProjectCommands::New {
                name,
                width,
                height,
                bleed,
                parent_dir,
                quiet,
            } => {
                let parent = parent_dir
                    .as_ref()
                    .map(|p| p.as_path())
                    .unwrap_or_else(|| std::path::Path::new("."));

                let config = commands::project::new::NewConfig {
                    name: name.clone(),
                    width_mm: *width,
                    height_mm: *height,
                    bleed_mm: *bleed,
                    quiet: *quiet,
                };

                let result = commands::project_new(parent, &config)?;

                println!("✅ Project '{}' created successfully!", name);
                println!("📁 Location: {}", result.project_root.display());
                println!("🌿 Branch: {}", result.branch);
                println!("📄 YAML: {}", result.yaml_path.display());
                println!("📝 Template: {}", result.typ_path.display());

                Ok(())
            }
        }
    }
}
