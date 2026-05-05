mod app;
mod background;
mod state;
mod task;
mod thumbnail;

use std::path::PathBuf;

use app::FotobuchApp;
use fotobuch::app_settings::AppSettings;
use fotobuch::vault::{ensure_vault, resolve_vault};

/// GUI-specific CLI arguments.
struct Args {
    /// Override vault directory (highest priority).
    vault: Option<PathBuf>,
}

fn parse_args() -> Args {
    let mut vault: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--vault" {
            vault = args.next().map(PathBuf::from);
        } else if let Some(p) = arg.strip_prefix("--vault=") {
            vault = Some(PathBuf::from(p));
        }
    }
    Args { vault }
}

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let args = parse_args();
    let settings = AppSettings::load().unwrap_or_default();

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let env_vault = std::env::var("FOTOBUCH_VAULT").ok();

    let vault_path = resolve_vault(args.vault.as_deref(), env_vault.as_deref(), &cwd, &settings);

    if let Err(e) = ensure_vault(&vault_path) {
        tracing::warn!(
            "Could not initialize vault at {}: {e}",
            vault_path.display()
        );
    }

    // First-run: no fotobuch projects in the vault yet.
    let show_welcome = fotobuch::commands::project::project_list(&vault_path)
        .map(|o| o.result.is_empty())
        .unwrap_or(true);

    eframe::run_native(
        "fotobuch",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(FotobuchApp::new(cc, vault_path, show_welcome)?))),
    )
}
