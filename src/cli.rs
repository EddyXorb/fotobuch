//! Command-line interface for the photobook solver.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

// Handler modules for each command
pub mod add;
pub mod build;
pub mod config;
pub mod history;
pub mod place;
pub mod project;
pub mod rebuild;
pub mod remove;
pub mod status;

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

        /// Only include photos whose XMP metadata matches this regex
        #[arg(long, value_name = "REGEX")]
        filter_xmp: Option<String>,

        /// Only include photos whose source path matches this regex pattern
        #[arg(long, value_name = "REGEX")]
        filter: Option<String>,

        /// Preview what would be added without writing anything
        #[arg(long, short = 'd')]
        dry: bool,

        /// Re-add photos whose path already exists but whose content has changed
        #[arg(long)]
        update: bool,
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

        /// Remove all photos that are not placed in any layout page
        #[arg(long, conflicts_with = "patterns")]
        unplaced: bool,
    },

    /// Show project status
    Status {
        /// Show detailed information for a specific page (1-based)
        page: Option<usize>,
    },

    /// Show resolved configuration with defaults
    Config,

    /// Show project change history
    History {
        /// Number of entries to show (0 = all)
        #[arg(short = 'n', default_value_t = 5)]
        count: usize,
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
        /// Project name
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

    /// List all photobook projects
    List,

    /// Switch to another photobook project
    Switch {
        /// Project name to switch to
        name: String,
    },
}

impl Execute for Commands {
    fn execute(&self) -> Result<()> {
        match self {
            Commands::Add {
                paths,
                allow_duplicates,
                filter_xmp,
                filter,
                dry,
                update,
            } => add::handle(paths.clone(), *allow_duplicates, filter_xmp.clone(), filter.clone(), *dry, *update),
            Commands::Build { release, pages } => build::handle(*release, pages.clone()),
            Commands::Rebuild {
                page,
                range_start,
                range_end,
                flex,
                all,
            } => rebuild::handle(*page, *range_start, *range_end, *flex, *all),
            Commands::Place { filter, into } => place::handle(filter.clone(), *into),
            Commands::Remove {
                patterns,
                keep_files,
                unplaced,
            } => remove::handle(patterns.clone(), *keep_files, *unplaced),
            Commands::Status { page } => status::handle(*page),
            Commands::Config => config::handle(),
            Commands::History { count } => history::handle(*count),
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
            } => project::handle(project::ProjectSubcommand::New {
                name: name.clone(),
                width: *width,
                height: *height,
                bleed: *bleed,
                parent_dir: parent_dir.clone(),
                quiet: *quiet,
            }),
            ProjectCommands::List => project::handle(project::ProjectSubcommand::List),
            ProjectCommands::Switch { name } => {
                project::handle(project::ProjectSubcommand::Switch { name: name.clone() })
            }
        }
    }
}
