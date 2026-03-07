//! Configuration structures for the YAML project state.
//!
//! This module contains the configuration structures that are persisted in `fotobuch.yaml`
//! and also used internally throughout the application to minimize translation overhead.

mod fitness_weights;

use serde::{Deserialize, Serialize};

/// Complete project configuration as persisted in YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub book: BookConfig,
    #[serde(default)]
    pub ga: GaConfig,
    #[serde(default)]
    pub preview: PreviewConfig,
}

/// Book-specific configuration (page dimensions, bleed, margins, gaps)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookConfig {
    pub title: String,
    pub page_width_mm: f64,
    pub page_height_mm: f64,
    pub bleed_mm: f64,
    #[serde(default = "default_margin_mm")]
    pub margin_mm: f64,
    #[serde(default = "default_gap_mm")]
    pub gap_mm: f64,
    #[serde(default = "default_bleed_threshold_mm")]
    pub bleed_threshold_mm: f64,
}

fn default_margin_mm() -> f64 {
    10.0
}

fn default_gap_mm() -> f64 {
    5.0
}

fn default_bleed_threshold_mm() -> f64 {
    3.0
}

/// Genetic algorithm configuration (persisted in YAML, mirrors internal GaConfig)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaConfig {
    #[serde(default = "default_seed")]
    pub seed: u64,
    #[serde(default = "default_population_size")]
    pub population_size: usize,
    #[serde(default = "default_max_generations")]
    pub max_generations: usize,
    #[serde(default = "default_mutation_rate")]
    pub mutation_rate: f64,
    #[serde(default = "default_crossover_rate")]
    pub crossover_rate: f64,
    #[serde(default = "default_elite_count")]
    pub elite_count: usize,
    #[serde(default = "default_no_improvement_limit")]
    pub no_improvement_limit: Option<usize>,
    #[serde(default)]
    pub weights: fitness_weights::FitnessWeights,

    /// Number of islands (independent populations).
    #[serde(default = "default_islands_nr")]
    pub islands_nr: usize,

    /// Generations between migrations.
    #[serde(default = "default_islands_migration_interval")]
    pub islands_migration_interval: usize,

    /// Number of individuals to migrate per island per migration event.
    #[serde(default = "default_islands_nr_migrants")]
    pub islands_nr_migrants: usize,
}

impl Default for GaConfig {
    fn default() -> Self {
        Self {
            islands_nr: default_islands_nr(),
            islands_migration_interval: default_islands_migration_interval(),
            islands_nr_migrants: default_islands_nr_migrants(),
            seed: default_seed(),
            population_size: default_population_size(),
            max_generations: default_max_generations(),
            mutation_rate: default_mutation_rate(),
            crossover_rate: default_crossover_rate(),
            elite_count: default_elite_count(),
            no_improvement_limit: default_no_improvement_limit(),
            weights: fitness_weights::FitnessWeights::default(),
        }
    }
}

fn default_islands_migration_interval() -> usize {
    5
}

fn default_islands_nr_migrants() -> usize {
    2
}

fn default_islands_nr() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

fn default_seed() -> u64 {
    42
}

fn default_population_size() -> usize {
    200
}

fn default_max_generations() -> usize {
    1000
}

fn default_mutation_rate() -> f64 {
    0.3
}

fn default_crossover_rate() -> f64 {
    0.7
}

fn default_elite_count() -> usize {
    20
}

fn default_no_improvement_limit() -> Option<usize> {
    Some(15)
}

/// Preview-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewConfig {
    #[serde(default = "default_show_filenames")]
    pub show_filenames: bool,
    #[serde(default = "default_show_page_numbers")]
    pub show_page_numbers: bool,
    #[serde(default = "default_max_preview_px")]
    pub max_preview_px: u32,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            show_filenames: default_show_filenames(),
            show_page_numbers: default_show_page_numbers(),
            max_preview_px: default_max_preview_px(),
        }
    }
}

fn default_show_filenames() -> bool {
    true
}

fn default_show_page_numbers() -> bool {
    true
}

fn default_max_preview_px() -> u32 {
    800
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ga_config_default() {
        let config = GaConfig::default();
        assert_eq!(config.population_size, 200);
        assert_eq!(config.max_generations, 1000);
        assert!(config.islands_nr >= 1);
        assert!(config.islands_nr >= 1);
        assert_eq!(config.islands_migration_interval, 5);
        assert_eq!(config.islands_nr_migrants, 2);
    }
}
