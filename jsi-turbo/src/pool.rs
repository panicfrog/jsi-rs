//! A minimal shared worker pool for async module bodies (no tokio
//! dependency; futures here never really suspend, a poll loop suffices).

use std::sync::mpsc;
use std::sync::OnceLock;

type Job = Box<dyn FnOnce() + Send + 'static>;

pub fn spawn(job: Job) {
    pool().submit(job);
}

struct Pool {
    sender: mpsc::Sender<Job>,
}

impl Pool {
    fn new() -> Self {
        const WORKERS: usize = 4;
        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = std::sync::Arc::new(std::sync::Mutex::new(receiver));
        for _ in 0..WORKERS {
            let receiver = receiver.clone();
            std::thread::Builder::new()
                .name("jsi-turbo-worker".into())
                .spawn(move || loop {
                    let job = { receiver.lock().unwrap().recv() };
                    match job {
                        Ok(job) => job(),
                        Err(_) => return,
                    }
                })
                .expect("failed to spawn jsi-turbo worker thread");
        }
        Self { sender }
    }

    fn submit(&self, job: Job) {
        // a closed channel (workers gone, e.g. process teardown) is not
        // worth panicking over
        let _ = self.sender.send(job);
    }
}

fn pool() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(Pool::new)
}
