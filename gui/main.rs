mod app;
mod background;
mod state;
mod task;

use app::FotobuchApp;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let project_root = std::env::current_dir().expect("cannot determine current directory");

    eframe::run_native(
        "fotobuch",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(FotobuchApp::new(cc, project_root)?))),
    )
}
