//! Command-line interface for the photobook solver.

use clap::Parser;
use std::path::PathBuf;

/// Photobook layout solver: optimizes photo placement on a canvas using genetic algorithms.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Root directory containing photo subdirectories
    #[arg(short, long)]
    pub input: PathBuf,

    /// Output file path (extension determines format: .json, .typ, or .pdf)
    #[arg(short, long, default_value = "layout.json")]
    pub output: PathBuf,

    // === Canvas Parameters ===
    /// Canvas width in mm
    #[arg(long, default_value_t = 297.0)]
    pub width: f64,

    /// Canvas height in mm
    #[arg(long, default_value_t = 210.0)]
    pub height: f64,

    /// Gap between photos in mm
    #[arg(long, default_value_t = 5.0)]
    pub beta: f64,

    /// Bleed over paper edge in mm
    #[arg(long, default_value_t = 0.0)]
    pub bleed: f64,

    // === GA Parameters ===
    /// Population size per island
    #[arg(long, default_value_t = 300)]
    pub population: usize,

    /// Maximum generations
    #[arg(long, default_value_t = 100)]
    pub generations: usize,

    /// Mutation rate (0.0-1.0)
    #[arg(long, default_value_t = 0.2)]
    pub mutation_rate: f64,

    /// Crossover rate (0.0-1.0)
    #[arg(long, default_value_t = 0.7)]
    pub crossover_rate: f64,

    // === Island Model Parameters ===
    /// Number of islands (default: number of CPU cores)
    #[arg(long)]
    pub islands: Option<usize>,

    /// Generations between migrations
    #[arg(long, default_value_t = 5)]
    pub migration_interval: usize,

    /// Number of migrants per migration
    #[arg(long, default_value_t = 2)]
    pub migrants: usize,

    /// Timeout in seconds (0 = no timeout)
    #[arg(long, default_value_t = 30)]
    pub timeout: u64,

    /// Random seed for reproducibility
    #[arg(long)]
    pub seed: Option<u64>,

    // === Fitness Weights ===
    /// Weight for size distribution cost
    #[arg(long, default_value_t = 1.0)]
    pub w_size: f64,

    /// Weight for canvas coverage cost
    #[arg(long, default_value_t = 0.15)]
    pub w_coverage: f64,

    /// Weight for barycenter cost
    #[arg(long, default_value_t = 0.5)]
    pub w_barycenter: f64,

    /// Weight for reading order cost
    #[arg(long, default_value_t = 0.3)]
    pub w_order: f64,

    /// Verbose output (progress and fitness)
    #[arg(short, long)]
    pub verbose: bool,
}
