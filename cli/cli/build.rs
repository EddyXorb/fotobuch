//! Handler for `fotobuch build` command

use anyhow::Context;
use anyhow::Result;
use fotobuch::commands;
use fotobuch::commands::build::{BuildConfig, BuildPlan};

pub fn handle(pages: Option<Vec<usize>>) -> Result<()> {
    let project_root = std::env::current_dir().context("Failed to determine current directory")?;

    let config = BuildConfig {
        plan: BuildPlan::Auto { pages },
        skip_pdf: false,
        skip_cache_update: false,
    };

    commands::build::build(&project_root, &config)?;

    Ok(())
}

pub fn handle_release(force: bool) -> Result<()> {
    let project_root = std::env::current_dir().context("Failed to determine current directory")?;

    let config = BuildConfig {
        plan: BuildPlan::Release { force },
        skip_pdf: false,
        skip_cache_update: false,
    };

    commands::build::build(&project_root, &config)?;

    Ok(())
}
