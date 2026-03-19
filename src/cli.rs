//! Command-line interface for the photobook solver.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

// Handler modules for each command
pub mod add;
pub mod build;
pub mod config;
pub mod history;
pub mod page;
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

        /// Only include photos whose XMP metadata matches this regex (can be repeated, all must match)
        #[arg(long, value_name = "REGEX")]
        filter_xmp: Vec<String>,

        /// Only include photos whose source path matches this regex pattern (can be repeated, all must match)
        #[arg(long, value_name = "REGEX")]
        filter: Vec<String>,

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
        /// Only place photos matching this regex pattern (can be repeated, all must match)
        #[arg(long, value_name = "REGEX")]
        filter: Vec<String>,

        /// Place all matching photos onto this specific page (1-based)
        #[arg(long)]
        into: Option<usize>,
    },

    /// Remove photos from the layout at a page:slot address (they stay in the project)
    ///
    /// The page is NOT deleted automatically, even if it becomes empty.
    /// To delete a whole page and unplace its photos, use: page move PAGE ->
    Unplace {
        /// Slot address: "3:2" (slot 2 on page 3), "3:2,7", "3:2..5", "3:2..5,7"
        address: String,
    },

    /// Page manipulation commands (move, split, combine, swap)
    Page {
        #[command(subcommand)]
        command: PageCommands,
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

/// Page subcommands
#[derive(Subcommand, Debug)]
pub enum PageCommands {
    /// Move, swap, or unplace photos between pages
    ///
    /// Three forms:
    ///   SRC -> DST    Move to another page (source page stays, even if empty)
    ///   SRC <> DST    Swap between two addresses
    ///   SRC ->        Unplace (no destination): pages deleted, slots emptied
    ///
    /// Addressing:
    ///   3             Whole page
    ///   3,5  3..5     Multiple pages
    ///   3:2           Single slot on page 3
    ///   3:1..3,7      Slots 1-3 and 7 on page 3
    ///   4+            New page after page 4 (move destination only)
    ///
    /// Move:
    ///   3:2 -> 5      Slot 2 from page 3 to page 5
    ///   3,4 -> 5      Merge pages 3 and 4 into page 5
    ///   3:2 -> 4+     Slot 2 onto a new page inserted after page 4
    ///
    /// Swap:
    ///   3:2 <> 5:6    Swap single slots
    ///   3:1,4 <> 5:2..5  Slot swap with differing counts
    ///   3 <> 5        Swap entire pages
    ///   3..6 <> 8..11 Swap page ranges (pairwise, equal count, no overlap)
    ///
    /// Unplace:
    ///   3 ->          Delete page 3, photos become unplaced
    ///   3:2 ->        Unplace slot 2, page 3 stays (possibly empty)
    #[command(verbatim_doc_comment)]
    Move {
        /// Expression passed as space-separated tokens, e.g.: 3:2 -> 5
        #[arg(num_args = 1..)]
        args: Vec<String>,
    },
    /// Split a page at a slot: photos from that slot onwards move to a new page inserted after
    ///
    /// Shortcut for: page move PAGE:SLOT.. -> PAGE+
    /// Error if SLOT is the first slot (would leave the original page empty).
    Split {
        /// Address "PAGE:SLOT", e.g. "3:4" splits page 3 at slot 4
        address: String,
    },
    /// Merge pages onto the first one, then delete the now-empty source pages
    ///
    /// All following page numbers shift down accordingly.
    Combine {
        /// Pages expression: "3,5" (page 5 onto 3) or "3..5" (pages 4-5 onto 3)
        pages: String,
    },
    /// Swap photos between two addresses (shortcut for: page move A <> B)
    ///
    /// Supports the same addressing as "page move <>":
    ///   3:2   5:6          Single slot swap
    ///   3:1..3  5:2..4     Slot range swap
    ///   3  5               Whole page swap
    ///   3..6  8..11        Page range swap (pairwise: 3↔8, 4↔9, …)
    ///   3,5  7,9           Page list swap (3↔7, 5↔9)
    ///
    /// Page range/list swaps require equal counts and no overlap between sides.
    #[command(verbatim_doc_comment)]
    Swap {
        /// Left address: "3:2", "3:1..3", "3", "3..6", "3,5"
        left: String,
        /// Right address: "5:6", "5:2..4", "5", "8..11", "7,9"
        right: String,
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
            } => add::handle(
                paths.clone(),
                *allow_duplicates,
                filter_xmp.to_vec(),
                filter.to_vec(),
                *dry,
                *update,
            ),
            Commands::Build { release, pages } => build::handle(*release, pages.clone()),
            Commands::Rebuild {
                page,
                range_start,
                range_end,
                flex,
                all,
            } => rebuild::handle(*page, *range_start, *range_end, *flex, *all),
            Commands::Place { filter, into } => place::handle(filter.to_vec(), *into),
            Commands::Unplace { address } => page::handle_unplace(address),
            Commands::Page { command } => command.execute(),
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

impl Execute for PageCommands {
    fn execute(&self) -> Result<()> {
        match self {
            PageCommands::Move { args } => page::handle_move(args),
            PageCommands::Split { address } => page::handle_split(address),
            PageCommands::Combine { pages } => page::handle_combine(pages),
            PageCommands::Swap { left, right } => page::handle_swap(left, right),
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
