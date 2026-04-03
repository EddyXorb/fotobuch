//! Command-line interface for the photobook solver.

use anyhow::Result;
use clap::{ArgGroup, CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
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
pub mod undo;

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

        /// Scan directories recursively (each subdir becomes its own group)
        #[arg(long, short = 'r')]
        recursive: bool,

        /// Area weight for all imported photos (default: 1.0)
        #[arg(long, value_name = "WEIGHT", default_value_t = 1.0)]
        weight: f64,
    },

    /// Calculate layout and generate preview or final PDF
    Build {
        /// Release subcommand (generate final PDF instead of preview)
        #[command(subcommand)]
        command: Option<BuildCommands>,

        /// Only rebuild specific pages (0-based, comma-separated or repeated flag)
        #[arg(long, value_delimiter = ',')]
        pages: Option<Vec<usize>>,
    },

    /// Force re-optimization of pages or page ranges
    Rebuild {
        /// Single page to rebuild (0-based index)
        #[arg(long, conflicts_with_all = ["range_start", "all"])]
        page: Option<usize>,

        /// Start of page range (0-based index, requires --range-end)
        #[arg(long, requires = "range_end", conflicts_with_all = ["page", "all"])]
        range_start: Option<usize>,

        /// End of page range (0-based index, inclusive, requires --range-start)
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

        /// Place all matching photos onto this specific page (0-based index)
        #[arg(long)]
        into: Option<usize>,
    },

    /// Remove photos from the layout at a page:slot address (they stay in the project)
    ///
    /// The page is deleted automatically if it becomes empty.
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
        /// Show detailed information for a specific page (0-based index)
        page: Option<usize>,
    },

    /// Configuration commands (show or mutate)
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Show project change history
    History {
        /// Number of entries to show (0 = all)
        #[arg(short = 'n', default_value_t = 5)]
        count: usize,
    },

    /// Undo the last N commits (default: 1)
    Undo {
        /// Number of steps to undo
        #[arg(default_value_t = 1)]
        steps: usize,
    },

    /// Redo N previously undone commits (default: 1)
    Redo {
        /// Number of steps to redo
        #[arg(default_value_t = 1)]
        steps: usize,
    },
    /// Project management commands
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },

    /// Create a new photobook project (alias for `project new`)
    Init {
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
        /// Create project with an active cover page
        #[arg(long, default_value_t = false)]
        with_cover: bool,
        /// Cover width in millimeters
        #[arg(long, requires = "with_cover")]
        cover_width: Option<f64>,
        /// Cover height in millimeters
        #[arg(long, requires = "with_cover")]
        cover_height: Option<f64>,
        /// Spine width growth per 10 inner pages in mm
        #[arg(long, requires = "with_cover", conflicts_with = "spine_mm")]
        spine_grow_per_10_pages_mm: Option<f64>,
        /// Fixed spine width in mm
        #[arg(
            long,
            requires = "with_cover",
            conflicts_with = "spine_grow_per_10_pages_mm"
        )]
        spine_mm: Option<f64>,
        /// Inner margin in millimeters (default: 0)
        #[arg(long, default_value_t = 0.0)]
        margin_mm: f64,
    },

    /// Print shell completion script to stdout
    ///
    /// Usage:
    ///   fotobuch completions --shell bash   >> ~/.bash_completion
    ///   fotobuch completions --shell zsh    >> ~/.zshrc
    ///   fotobuch completions --shell fish   > ~/.config/fish/completions/fotobuch.fish
    ///   fotobuch completions --shell powershell >> $PROFILE
    #[command(verbatim_doc_comment)]
    Completions {
        /// Shell to generate completions for
        #[arg(long, value_enum)]
        shell: Shell,
    },
}

/// Build subcommands
#[derive(Subcommand, Debug)]
pub enum BuildCommands {
    /// Generate final high-quality PDF at 300 DPI
    Release {
        /// Force release even if layout has uncommitted changes
        #[arg(long)]
        force: bool,
    },
}

/// Page subcommands
#[derive(Subcommand, Debug)]
pub enum PageCommands {
    /// Move or unplace photos between pages
    ///
    /// Two forms: "SRC to DST" (move) and "SRC out" (unplace).
    ///
    /// Addressing: 3 = whole page, 3:2 = slot 2 on page 3,
    /// 3:1..3,7 = slots 1-3 and 7, 4+ = new page after 4.
    ///
    /// Move examples: "3:2 to 5", "3,4 to 5", "3:2 to 4+".
    /// Unplace examples: "3 out", "3:2 out".
    ///
    /// See the documentation for the full addressing syntax.
    #[command(verbatim_doc_comment)]
    Move {
        /// Expression passed as space-separated tokens, e.g.: 3:2 to 5
        #[arg(num_args = 1..)]
        args: Vec<String>,
    },
    /// Split a page at a slot: photos from that slot onwards move to a new page inserted after
    ///
    /// Shortcut for `page move PAGE:SLOT.. to PAGE+`.
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
    /// Swap photos between two addresses (only single numbers or ranges, no comma lists)
    ///
    /// Page swap: "3 5" swaps pages, "1..2 5..9" swaps blocks.
    /// Slot swap: "3:2 5:6" swaps individual slots,
    /// "3:2..4 5:6..9" swaps slot ranges (different sizes ok).
    ///
    /// Errors on overlapping ranges or comma-separated lists as operands.
    #[command(verbatim_doc_comment)]
    Swap {
        /// Left address: "3:2", "3:1..3", "3", "3..6"
        left: String,
        /// Right address: "5:6", "5:2..4", "5", "8..11"
        right: String,
    },
    /// Show photo metadata for slots on a page
    ///
    /// Address forms: `3` (all slots), `3:2` (single slot), `3:1..3,7` (slots 1–3 and 7).
    ///
    /// Without flags: full table (or vertical view for a single slot).
    /// With a flag: machine-readable single-field output.
    Info {
        /// Address: "3", "3:2", "3:1..3,7"
        address: String,
        /// Output only area weights (format: page:slot=weight)
        #[arg(long)]
        weights: bool,
        /// Output only photo IDs
        #[arg(long)]
        ids: bool,
        /// Output only pixel dimensions
        #[arg(long)]
        pixels: bool,
    },
    /// Set area_weight for one or more slots
    ///
    /// Examples: `3:2 2.0` (single slot), `3:1..3,7 2.0` (multiple slots), `3 2.0` (whole page).
    Weight {
        /// Address: "3", "3:2", "3:1..3,7"
        address: String,
        /// Weight value (must be > 0)
        weight: f64,
    },
    /// Toggle page mode between auto (solver) and manual (user-placed)
    ///
    /// Syntax: `fotobuch page mode <pages> <a|m|auto|manual>`
    ///
    /// Examples: `3 m` (page 3 to manual), `3..5 a` (pages 3-5 to auto).
    Mode {
        /// Pages to change: "3", "3..5", "3,5"
        pages: String,
        /// Mode: 'a' or 'auto' for auto-solver, 'm' or 'manual' for manual placement
        mode: String,
    },
    /// Reposition or rescale slots on a Manual-mode page.
    ///
    /// Syntax: `fotobuch page pos <address> [--by dx,dy] [--at x,y] [--scale s]`
    ///
    /// Examples:
    ///   `4:2 --by -20,30`          — move slot 2 on page 4 relatively
    ///   `4:2 --at 100,50`          — set slot 2 origin to (100mm, 50mm)
    ///   `4:2 --scale 1.5`          — scale slot 2 by 1.5×
    ///   `4:2..5 --by -20,30`       — move slots 2–5 together
    ///   `4:2 --at 100,50 --scale 2` — absolute position + scale
    ///
    /// At least one of --by, --at, --scale is required. --by and --at are mutually exclusive.
    /// The page must be in manual mode.
    #[command(group(ArgGroup::new("movement").required(true).args(["by", "at", "scale"])))]
    Pos {
        /// Address: "4:2", "4:2..5", "4:1,3"
        address: String,
        /// Relative move in mm: "dx,dy" (e.g. "-20,30")
        #[arg(long, conflicts_with = "at")]
        by: Option<String>,
        /// Absolute position in mm: "x,y" (e.g. "100,50")
        #[arg(long, conflicts_with = "by")]
        at: Option<String>,
        /// Scale factor applied to width and height (origin stays fixed)
        #[arg(long)]
        scale: Option<f64>,
    },
}

/// Config subcommands
#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Show resolved configuration with defaults
    Show,
    /// Set a config value using dot-notation (e.g. `book.dpi 300`)
    ///
    /// Supported keys mirror the YAML config hierarchy.
    /// Types are auto-detected: true/false → bool, integers → int, decimals → float, else string.
    Set {
        /// Dot-notation key, e.g. "book.dpi" or "book.cover.active"
        key: String,
        /// New value, e.g. "300", "true", "3.5", "spread"
        value: String,
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

        /// Create project with an active cover page
        #[arg(long, default_value_t = false)]
        with_cover: bool,

        /// Cover width in millimeters (defaults to page_width * 2 if --with-cover is set, with warning)
        #[arg(long, requires = "with_cover")]
        cover_width: Option<f64>,

        /// Cover height in millimeters (defaults to page_height if --with-cover is set, with warning)
        #[arg(long, requires = "with_cover")]
        cover_height: Option<f64>,

        /// Spine width growth per 10 inner pages in mm (auto mode, conflicts with --spine-mm)
        #[arg(long, requires = "with_cover", conflicts_with = "spine_mm")]
        spine_grow_per_10_pages_mm: Option<f64>,

        /// Fixed spine width in mm (conflicts with --spine-grow-per-10-pages-mm)
        #[arg(
            long,
            requires = "with_cover",
            conflicts_with = "spine_grow_per_10_pages_mm"
        )]
        spine_mm: Option<f64>,
        /// Inner margin in millimeters (default: 0)
        #[arg(long, default_value_t = 0.0)]
        margin_mm: f64,
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
                recursive,
                weight,
            } => add::handle(add::AddArgs {
                paths: paths.clone(),
                allow_duplicates: *allow_duplicates,
                filter_xmp: filter_xmp.to_vec(),
                filter: filter.to_vec(),
                dry: *dry,
                update: *update,
                recursive: *recursive,
                weight: *weight,
            }),
            Commands::Build { command, pages } => match command {
                Some(BuildCommands::Release { force }) => build::handle_release(*force),
                None => build::handle(false, pages.clone()),
            },
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
            Commands::Config { command } => match command {
                ConfigCommands::Show => config::handle_show(),
                ConfigCommands::Set { key, value } => config::handle_set(key, value),
            },
            Commands::History { count } => history::handle(*count),
            Commands::Undo { steps } => undo::handle_undo(*steps),
            Commands::Redo { steps } => undo::handle_redo(*steps),
            Commands::Project { command } => command.execute(),
            Commands::Init {
                name,
                width,
                height,
                bleed,
                parent_dir,
                quiet,
                with_cover,
                cover_width,
                cover_height,
                spine_grow_per_10_pages_mm,
                spine_mm,
                margin_mm,
            } => project::handle(project::ProjectSubcommand::New {
                name: name.clone(),
                width: *width,
                height: *height,
                bleed: *bleed,
                parent_dir: parent_dir.clone(),
                quiet: *quiet,
                with_cover: *with_cover,
                cover_width: *cover_width,
                cover_height: *cover_height,
                spine_grow_per_10_pages_mm: *spine_grow_per_10_pages_mm,
                spine_mm: *spine_mm,
                margin_mm: *margin_mm,
            }),
            Commands::Completions { shell } => {
                clap_complete::generate(
                    *shell,
                    &mut Cli::command(),
                    "fotobuch",
                    &mut std::io::stdout(),
                );
                Ok(())
            }
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
            PageCommands::Info {
                address,
                weights,
                ids,
                pixels,
            } => {
                use fotobuch::commands::page::InfoFilter;
                page::handle_info(
                    address,
                    InfoFilter {
                        weights: *weights,
                        ids: *ids,
                        pixels: *pixels,
                    },
                )
            }
            PageCommands::Weight { address, weight } => page::handle_weight(address, *weight),
            PageCommands::Mode { pages, mode } => page::handle_mode(pages, mode),
            PageCommands::Pos {
                address,
                by,
                at,
                scale,
            } => page::handle_pos(address, by.as_deref(), at.as_deref(), *scale),
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
                with_cover,
                cover_width,
                cover_height,
                spine_grow_per_10_pages_mm,
                spine_mm,
                margin_mm,
            } => project::handle(project::ProjectSubcommand::New {
                name: name.clone(),
                width: *width,
                height: *height,
                bleed: *bleed,
                parent_dir: parent_dir.clone(),
                quiet: *quiet,
                with_cover: *with_cover,
                cover_width: *cover_width,
                cover_height: *cover_height,
                spine_grow_per_10_pages_mm: *spine_grow_per_10_pages_mm,
                spine_mm: *spine_mm,
                margin_mm: *margin_mm,
            }),
            ProjectCommands::List => project::handle(project::ProjectSubcommand::List),
            ProjectCommands::Switch { name } => {
                project::handle(project::ProjectSubcommand::Switch { name: name.clone() })
            }
        }
    }
}
