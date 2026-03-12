//! Handler for `fotobuch build` command

use anyhow::Context;
use anyhow::Result;
use photobook_solver::commands;

pub fn handle(release: bool, pages: Option<Vec<usize>>) -> Result<()> {
    let project_root = std::env::current_dir().context("Failed to determine current directory")?;

    let config = commands::build::BuildConfig { release, pages };

    let result = commands::build::build(&project_root, &config)?;

    commands::build::print_build_result(&result);

    Ok(())
}
