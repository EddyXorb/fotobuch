//! Configuration structures for the YAML project state.
//!
//! This module contains the configuration structures that are persisted in `fotobuch.yaml`
//! and also used internally throughout the application to minimize translation overhead.

use serde::{Deserialize, Serialize};

/// Complete project configuration as persisted in YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub book: BookConfig,
    #[serde(default)]
    pub ga: GaConfigYaml,
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
pub struct GaConfigYaml {
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
    pub weights: FitnessWeightsYaml,
}

impl Default for GaConfigYaml {
    fn default() -> Self {
        Self {
            seed: default_seed(),
            population_size: default_population_size(),
            max_generations: default_max_generations(),
            mutation_rate: default_mutation_rate(),
            crossover_rate: default_crossover_rate(),
            elite_count: default_elite_count(),
            no_improvement_limit: default_no_improvement_limit(),
            weights: FitnessWeightsYaml::default(),
        }
    }
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

/// Fitness weights configuration (persisted in YAML)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessWeightsYaml {
    #[serde(default = "default_area_usage")]
    pub area_usage: f64,
    #[serde(default = "default_aspect_preservation")]
    pub aspect_preservation: f64,
    #[serde(default = "default_bleed_penalty")]
    pub bleed_penalty: f64,
    #[serde(default = "default_alignment")]
    pub alignment: f64,
}

impl Default for FitnessWeightsYaml {
    fn default() -> Self {
        Self {
            area_usage: default_area_usage(),
            aspect_preservation: default_aspect_preservation(),
            bleed_penalty: default_bleed_penalty(),
            alignment: default_alignment(),
        }
    }
}

fn default_area_usage() -> f64 {
    1.0
}

fn default_aspect_preservation() -> f64 {
    1.0
}

fn default_bleed_penalty() -> f64 {
    1.0
}

fn default_alignment() -> f64 {
    0.5
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
