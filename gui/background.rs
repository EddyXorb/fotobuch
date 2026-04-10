use std::path::{Path, PathBuf};
use std::time::Instant;

use crossbeam::channel::{Receiver, Sender, unbounded};
use fotobuch::output::typst::render_pages;

use crate::task::{BackgroundResult, BackgroundTask};

pub fn spawn(
    project_root: PathBuf,
    project_name: String,
) -> (Sender<BackgroundTask>, Receiver<BackgroundResult>) {
    let (task_tx, task_rx) = unbounded::<BackgroundTask>();
    let (result_tx, result_rx) = unbounded::<BackgroundResult>();

    std::thread::spawn(move || {
        let pool = build_pool();
        while let Ok(task) = task_rx.recv() {
            match task {
                BackgroundTask::RenderPages {
                    pages,
                    pixel_per_pt,
                } => {
                    for page in pages {
                        let root = project_root.clone();
                        let name = project_name.clone();
                        let tx = result_tx.clone();
                        pool.spawn(move || render_page(&root, &name, page, pixel_per_pt, &tx));
                    }
                }
            }
        }
    });

    (task_tx, result_rx)
}

fn build_pool() -> rayon::ThreadPool {
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .saturating_sub(1)
        .max(1);
    rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .expect("failed to build render thread pool")
}

fn render_page(
    project_root: &Path,
    project_name: &str,
    page: usize,
    pixel_per_pt: f32,
    result_tx: &Sender<BackgroundResult>,
) {
    let t = Instant::now();
    match render_pages(project_root, project_name, &[page], pixel_per_pt) {
        Ok(rendered) => {
            let duration = t.elapsed();
            for r in rendered {
                let _ = result_tx.send(BackgroundResult::PageRendered { page: r, duration });
            }
        }
        Err(e) => {
            let _ = result_tx.send(BackgroundResult::Error(e.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crossbeam::channel::RecvTimeoutError;
    use tempfile::TempDir;

    use super::*;
    use crate::task::BackgroundResult;

    #[test]
    fn worker_renders_single_page() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("test.typ"),
            "#set page(width: 50mm, height: 50mm)\nHello",
        )
        .unwrap();

        let (task_tx, result_rx) = spawn(temp.path().to_owned(), "test".to_owned());
        task_tx
            .send(BackgroundTask::RenderPages {
                pages: vec![0],
                pixel_per_pt: 1.0,
            })
            .unwrap();

        let result = result_rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("no result within timeout");

        match result {
            BackgroundResult::PageRendered { page: r, duration } => {
                assert_eq!(r.page, 0);
                assert!(!r.pixels.is_empty());
                assert!(duration.as_secs() < 30, "render took unexpectedly long");
            }
            BackgroundResult::Error(e) => panic!("worker error: {e}"),
        }

        match result_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {}
            Ok(_) => panic!("unexpected second result"),
        }
    }
}
