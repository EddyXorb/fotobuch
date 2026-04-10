mod app;
mod background;
mod state;
mod task;

use app::FotobuchApp;
use fotobuch::state_manager::StateManager;
use state::GuiState;
use task::BackgroundTask;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let project_root = std::env::current_dir().expect("cannot determine current directory");

    let mgr = match StateManager::open(&project_root) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error loading project: {e}");
            std::process::exit(1);
        }
    };

    let project_name = mgr.project_name().to_owned();
    let num_pages = mgr.state.layout.len();
    let page_width_mm = mgr.state.config.book.page_width_mm;
    let page_height_mm = mgr.state.config.book.page_height_mm;
    drop(mgr);

    let gui_state = GuiState::new(num_pages);
    let (task_tx, result_rx) = background::spawn(project_root, project_name);

    let _ = task_tx.send(BackgroundTask::RenderPages {
        pages: (0..num_pages).collect(),
        pixel_per_pt: 1.5,
    });

    eframe::run_native(
        "fotobuch",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            Ok(Box::new(FotobuchApp::new(
                cc,
                gui_state,
                task_tx,
                result_rx,
                page_width_mm,
                page_height_mm,
            )))
        }),
    )
}
