//! CLI-layer error hints.
//!
//! Translates typed lib errors into surface-specific remediation text.
//! Flag names are read from the live clap tree so renaming a flag breaks the test.

use clap::{Command, CommandFactory};
use fotobuch::commands::build::BuildError;
use fotobuch::commands::project::ProjectError;

use super::Cli;

/// Returns a hint string for a known typed lib error, or `None` for unknown errors.
pub fn hint_for(err: &anyhow::Error) -> Option<String> {
    if let Some(e) = err.downcast_ref::<BuildError>() {
        return Some(build_hint(e));
    }
    if let Some(e) = err.downcast_ref::<ProjectError>() {
        return Some(project_hint(e));
    }
    None
}

fn build_hint(err: &BuildError) -> String {
    let cmd = Cli::command();
    match err {
        BuildError::NoLayout => "run `fotobuch build` first".to_string(),
        BuildError::LayoutDirty { .. } => {
            let force = flag_long(&cmd, &["build", "release"], "force");
            format!("run `fotobuch build` first, or `fotobuch build release {force}` to force")
        }
        BuildError::PageIsManual { idx } => {
            format!("use `fotobuch page mode {idx} a` to switch to auto mode")
        }
        BuildError::CoverExcluded => {
            let page = flag_long(&cmd, &["rebuild"], "page");
            format!("use `fotobuch rebuild {page} 0` to rebuild the cover explicitly")
        }
    }
}

fn project_hint(err: &ProjectError) -> String {
    match err {
        ProjectError::NotFound { .. } => {
            "use `fotobuch project list` to see available projects".to_string()
        }
    }
}

/// Returns the `--flag-name` string for `arg_id` in the subcommand reached by `sub_path`.
///
/// Panics if the path or argument does not exist — caught by `hint_lookups_resolve`.
pub fn flag_long(cmd: &Command, sub_path: &[&str], arg_id: &str) -> String {
    let mut current = cmd.clone();
    for &sub in sub_path {
        current = {
            let found = current
                .get_subcommands()
                .find(|c| c.get_name() == sub)
                .unwrap_or_else(|| panic!("subcommand '{sub}' not found in hint lookup"));
            found.clone()
        };
    }
    let arg = current
        .get_arguments()
        .find(|a| a.get_id().as_str() == arg_id)
        .unwrap_or_else(|| panic!("argument '{arg_id}' not found in hint lookup"));
    format!(
        "--{}",
        arg.get_long()
            .unwrap_or_else(|| panic!("argument '{arg_id}' has no long flag"))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hint_lookups_resolve() {
        let cmd = Cli::command();
        flag_long(&cmd, &["build", "release"], "force");
        flag_long(&cmd, &["rebuild"], "page");
    }
}
