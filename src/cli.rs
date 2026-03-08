//! Command-line interface for the photobook solver.

use anyhow::Result;
use clap::{Parser, Subcommand};
use photobook_solver::commands;
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
        /// Directories containing photos to add
        paths: Vec<PathBuf>,

        /// Allow adding duplicate photos (by hash)
        #[arg(long)]
        allow_duplicates: bool,
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
    },
}

impl Execute for Commands {
    fn execute(&self) -> Result<()> {
        match self {
            Commands::Add {
                paths,
                allow_duplicates,
            } => commands::execute_add(paths.clone(), *allow_duplicates),
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
