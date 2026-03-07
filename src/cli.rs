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
}

impl Execute for Commands {
    fn execute(&self) -> Result<()> {
        match self {
            Commands::Add {
                paths,
                allow_duplicates,
            } => commands::execute_add(paths.clone(), *allow_duplicates),
        }
    }
}
