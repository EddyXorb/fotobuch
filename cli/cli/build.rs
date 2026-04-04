//! Handler for `fotobuch build` command

use anyhow::Context;
use anyhow::Result;
use fotobuch::commands;

pub fn handle(release: bool, pages: Option<Vec<usize>>) -> Result<()> {
    let project_root = std::env::current_dir().context("Failed to determine current directory")?;

    let config = commands::build::BuildConfig {
        release,
        force: false,
        pages,
    };

    let output = commands::build::build(&project_root, &config)?;

    commands::build::print_build_result(&output.result);

    Ok(())
}

pub fn handle_release(force: bool) -> Result<()> {
    let project_root = std::env::current_dir().context("Failed to determine current directory")?;

    let config = commands::build::BuildConfig {
        release: true,
        force,
        pages: None,
    };

    let output = commands::build::build(&project_root, &config)?;

    commands::build::print_build_result(&output.result);

    Ok(())
}
