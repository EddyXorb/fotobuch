use std::path::PathBuf;

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
        while let Ok(task) = task_rx.recv() {
            match task {
                BackgroundTask::RenderPages {
                    pages,
                    pixel_per_pt,
                } => {
                    for page in pages {
                        match render_pages(&project_root, &project_name, &[page], pixel_per_pt) {
                            Ok(rendered) => {
                                for r in rendered {
                                    let _ = result_tx.send(BackgroundResult::PageRendered(r));
                                }
                            }
                            Err(e) => {
                                let _ = result_tx.send(BackgroundResult::Error(e.to_string()));
                            }
                        }
                    }
                }
            }
        }
    });

    (task_tx, result_rx)
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
            BackgroundResult::PageRendered(r) => {
                assert_eq!(r.page, 0);
                assert!(!r.pixels.is_empty());
            }
            BackgroundResult::Error(e) => panic!("worker error: {e}"),
        }

        // No second message expected
        match result_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {}
            Ok(_) => panic!("unexpected second result"),
        }
    }
}
