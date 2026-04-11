use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crossbeam::channel::{Receiver, Sender, unbounded};
use fotobuch::output::typst_world::{TypstWorld, rasterize_page};

use crate::task::{BackgroundResult, BackgroundTask};

pub fn spawn(
    project_root: PathBuf,
    project_name: String,
) -> (Sender<BackgroundTask>, Receiver<BackgroundResult>) {
    let (task_tx, task_rx) = unbounded::<BackgroundTask>();
    let (result_tx, result_rx) = unbounded::<BackgroundResult>();

    std::thread::spawn(move || {
        let pool = build_pool();

        let mut world = match TypstWorld::new(&project_root, &project_name) {
            Ok(w) => w,
            Err(e) => {
                let _ = result_tx.send(BackgroundResult::Error(e.to_string()));
                return;
            }
        };

        while let Ok(task) = task_rx.recv() {
            match task {
                BackgroundTask::RenderPages {
                    pages,
                    pixel_per_pt,
                } => {
                    if let Err(e) = world.reload() {
                        let _ = result_tx.send(BackgroundResult::Error(e.to_string()));
                        continue;
                    }

                    let t_compile = Instant::now();
                    let doc = match world.compile_document() {
                        Ok(d) => d,
                        Err(e) => {
                            let _ = result_tx.send(BackgroundResult::Error(e.to_string()));
                            continue;
                        }
                    };
                    let compile_duration = t_compile.elapsed();

                    let doc = Arc::new(doc);
                    for page in pages {
                        let doc = Arc::clone(&doc);
                        let tx = result_tx.clone();
                        pool.spawn(move || {
                            let t_raster = Instant::now();
                            match rasterize_page(&doc, page, pixel_per_pt) {
                                Ok(rendered) => {
                                    let _ = tx.send(BackgroundResult::PageRendered {
                                        page: rendered,
                                        rasterize_duration: t_raster.elapsed(),
                                        compile_duration,
                                    });
                                }
                                Err(e) => {
                                    let _ = tx.send(BackgroundResult::Error(e.to_string()));
                                }
                            }
                        });
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Duration;

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
            .recv_timeout(Duration::from_secs(30))
            .expect("no result within timeout");

        match result {
            BackgroundResult::PageRendered {
                page: r,
                rasterize_duration,
                compile_duration,
            } => {
                assert_eq!(r.page, 0);
                assert!(!r.pixels.is_empty());
                assert!(rasterize_duration.as_secs() < 30);
                assert!(compile_duration.as_secs() < 30);
            }
            BackgroundResult::Error(e) => panic!("worker error: {e}"),
        }

        match result_rx.recv_timeout(Duration::from_millis(100)) {
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {}
            Ok(_) => panic!("unexpected second result"),
        }
    }
}
