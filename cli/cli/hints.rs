//! CLI-layer error hints.
//!
//! Translates typed lib errors into surface-specific remediation text.
//! Flag names are read from the live clap tree, so a renamed flag turns into a
//! missing hint — which `hints_exist_for_remediable_errors` reports as a failure.

use clap::{Command, CommandFactory};
use fotobuch::commands::build::BuildError;
use fotobuch::commands::page::{PageMoveError, ValidationError};
use fotobuch::commands::project::ProjectError;
use fotobuch::solver::CoverSolverError;
use fotobuch::state_manager::StateError;

use super::Cli;

/// Returns the remediation hint for a known typed lib error.
///
/// `None` means either an unknown error type or a condition the user cannot act
/// on in a single step.
pub fn hint_for(err: &anyhow::Error) -> Option<String> {
    if let Some(e) = err.downcast_ref::<BuildError>() {
        return build_hint(e);
    }
    if let Some(e) = err.downcast_ref::<ProjectError>() {
        return project_hint(e);
    }
    if let Some(e) = err.downcast_ref::<StateError>() {
        return state_hint(e);
    }
    if let Some(e) = err.downcast_ref::<CoverSolverError>() {
        return cover_hint(e);
    }
    if let Some(e) = err.downcast_ref::<PageMoveError>() {
        return page_hint(e);
    }
    None
}

fn build_hint(err: &BuildError) -> Option<String> {
    let cmd = Cli::command();
    match err {
        BuildError::NoLayout => Some("run `fotobuch build` first".to_string()),
        BuildError::LayoutDirty { .. } => {
            let force = flag_long(&cmd, &["build", "release"], "force")?;
            Some(format!(
                "run `fotobuch build` first, or `fotobuch build release {force}` to force"
            ))
        }
        BuildError::PageIsManual { idx } => Some(format!(
            "use `fotobuch page mode {idx} a` to switch to auto mode"
        )),
        BuildError::CoverExcluded => {
            let page = flag_long(&cmd, &["rebuild"], "page")?;
            Some(format!(
                "use `fotobuch rebuild {page} 0` to rebuild the cover explicitly"
            ))
        }
    }
}

fn project_hint(err: &ProjectError) -> Option<String> {
    match err {
        ProjectError::NotFound { .. } => {
            Some("use `fotobuch project list` to see available projects".to_string())
        }
        ProjectError::CoverWithoutSpine => {
            let cmd = Cli::command();
            let path = &["project", "new"];
            let grow = flag_long(&cmd, path, "spine_grow_per_10_pages_mm")?;
            let fixed = flag_long(&cmd, path, "spine_mm")?;
            let cover = flag_long(&cmd, path, "with_cover")?;
            Some(format!(
                "pass `{grow} <mm>` or `{fixed} <mm>` together with `{cover}`"
            ))
        }
    }
}

fn state_hint(err: &StateError) -> Option<String> {
    match err {
        StateError::NotOnProjectBranch { .. } => {
            Some("use `fotobuch project switch <name>` to check out a project".to_string())
        }
    }
}

fn cover_hint(err: &CoverSolverError) -> Option<String> {
    match err {
        CoverSolverError::MissingPhoto { .. } => {
            let cmd = Cli::command();
            let into = flag_long(&cmd, &["place"], "into")?;
            Some(format!(
                "assign the missing photo with `fotobuch place <photo> {into} 0`"
            ))
        }
    }
}

fn page_hint(err: &PageMoveError) -> Option<String> {
    match err {
        PageMoveError::Validation(e) => validation_hint(e),
        // `to_anyhow` unwraps this variant, so the inner error is normally
        // classified on its own; this arm only covers direct construction.
        PageMoveError::Other(inner) => hint_for(inner),
    }
}

/// Most validation errors state a condition the message already resolves
/// ("page 7 does not exist"); only a few have a next step worth naming.
fn validation_hint(err: &ValidationError) -> Option<String> {
    match err {
        ValidationError::PageNotManual(page) => Some(format!(
            "use `fotobuch page mode {page} m` to switch to manual mode"
        )),
        ValidationError::PageNotFound(_)
        | ValidationError::SlotNotFound { .. }
        | ValidationError::SlotEmpty { .. }
        | ValidationError::SwapRangesOverlap
        | ValidationError::SwapNonContiguous
        | ValidationError::CombineSinglePage(_)
        | ValidationError::SplitAtFirstSlot(_)
        | ValidationError::WeightOutOfRange(_) => None,
    }
}

/// Returns the `--flag-name` for `arg_id` in the subcommand reached by
/// `sub_path`, or `None` when the path or the argument does not exist.
///
/// A hint must never take the process down, so a failed lookup drops the hint
/// rather than panicking. The tests below turn such a lookup into a failure.
fn flag_long(cmd: &Command, sub_path: &[&str], arg_id: &str) -> Option<String> {
    let mut current = cmd;
    for &sub in sub_path {
        current = current.get_subcommands().find(|c| c.get_name() == sub)?;
    }
    let arg = current
        .get_arguments()
        .find(|a| a.get_id().as_str() == arg_id)?;
    Some(format!("--{}", arg.get_long()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One sample per error variant that carries a remediation.
    ///
    /// Every clap lookup used by a hint runs through here, so a renamed or
    /// removed flag surfaces as a missing hint instead of a wrong one.
    fn remediable_errors() -> Vec<anyhow::Error> {
        vec![
            BuildError::NoLayout.into(),
            BuildError::LayoutDirty { pages: vec![1, 2] }.into(),
            BuildError::PageIsManual { idx: 3 }.into(),
            BuildError::CoverExcluded.into(),
            ProjectError::NotFound {
                name: "urlaub".to_owned(),
            }
            .into(),
            ProjectError::CoverWithoutSpine.into(),
            StateError::NotOnProjectBranch {
                branch: "master".to_owned(),
            }
            .into(),
            CoverSolverError::MissingPhoto {
                index: 1,
                available: 0,
            }
            .into(),
            anyhow::Error::new(PageMoveError::Validation(ValidationError::PageNotManual(2))),
        ]
    }

    #[test]
    fn hints_exist_for_remediable_errors() {
        for err in remediable_errors() {
            assert!(
                hint_for(&err).is_some(),
                "no hint for `{err}` — a clap lookup probably stopped resolving"
            );
        }
    }

    #[test]
    fn unknown_errors_have_no_hint() {
        let err = anyhow::anyhow!("something the CLI knows nothing about");
        assert!(hint_for(&err).is_none());
    }

    #[test]
    fn validation_errors_without_a_next_step_have_no_hint() {
        let err = anyhow::Error::new(PageMoveError::Validation(ValidationError::PageNotFound(9)));
        assert!(hint_for(&err).is_none());
    }

    #[test]
    fn flag_long_returns_none_for_unknown_lookups() {
        let cmd = Cli::command();
        assert!(flag_long(&cmd, &["does-not-exist"], "force").is_none());
        assert!(flag_long(&cmd, &["build", "release"], "no-such-arg").is_none());
    }
}
