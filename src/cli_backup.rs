//! Command-line interface for the photobook solver.

use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

pub mod add;
pub mod build;
pub mod config;
pub mod history;
pub mod place;
pub mod project;
pub mod rebuild;
pub mod remove;
pub mod status;

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

    /// Show project status
    Status {
        /// Show detailed information for a specific page (1-based)
        page: Option<usize>,
    },

    /// Show resolved configuration with defaults
    Config,

    /// Show project change history
    History,

    /// Project management commands
    Project {
        #[command(subcommand)]
        command: project::ProjectCommands,
    },
}

impl Execute for Commands {
    fn execute(&self) -> Result<()> {
        match self {
            Commands::Add {
                paths,
                allow_duplicates,
            } => add::AddHandler {
                paths: paths.clone(),
                allow_duplicates: *allow_duplicates,
            }
            .execute(),
            Commands::Build { release, pages } => build::BuildHandler {
                release: *release,
                pages: pages.clone(),
            }
            .execute(),
            Commands::Rebuild {
                page,
                range_start,
                range_end,
                flex,
                all,
            } => rebuild::RebuildHandler {
                page: *page,
                range_start: *range_start,
                range_end: *range_end,
                flex: *flex,
                all: *all,
            }
            .execute(),
            Commands::Place { filter, into } => place::PlaceHandler {
                filter: filter.clone(),
                into: *into,
            }
            .execute(),
            Commands::Remove { patterns, keep_files } => remove::RemoveHandler {
                patterns: patterns.clone(),
                keep_files: *keep_files,
            }
            .execute(),
            Commands::Status { page } => status::StatusHandler { page: *page }.execute(),
            Commands::Config => config::ConfigHandler.execute(),
            Commands::History => history::HistoryHandler.execute(),
            Commands::Project { command } => command.execute(),
        }
    }
}
