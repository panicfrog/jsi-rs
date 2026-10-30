//! The jsi-turbo async executor — the "base runtime" for async method
//! bodies.
//!
//! A fixed pool of worker threads, each driving futures with
//! `futures_lite::future::block_on`: real waker semantics (a suspended
//! future parks its thread instead of spinning), so futures from the
//! smol-rs family work out of the box — `async-io`'s global reactor
//! thread drives timers/IO wakeups regardless of who runs the future.
//! Deliberately NOT included: a reactor, timers, IO, M:N multiplexing.
//! Module authors add what they need as ordinary dependencies
//! (`async-io`, `async-net`, or the `smol` facade). Tokio-ecosystem
//! libraries still require the module to hold its own
//! `tokio::runtime::Runtime`.
//!
//! A suspended future occupies its worker until woken — acceptable for
//! module workloads (a handful of concurrent bodies at any time). A
//! shared M:N executor was tried and hung on-device (fine on the host,
//! parked workers, task never dispatched; root cause unknown) — the
//! one-future-per-thread model is what is verified end to end.

use std::future::Future;
use std::sync::mpsc;
use std::sync::OnceLock;

type Job = Box<dyn FnOnce() + Send + 'static>;

/// Spawns a future onto the worker pool (detached: it runs to
/// completion on its own; nothing is joined).
pub fn spawn<F>(fut: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    pool().submit(Box::new(move || {
        futures_lite::future::block_on(fut);
    }));
}

struct Pool {
    sender: mpsc::Sender<Job>,
}

impl Pool {
    fn new() -> Self {
        // cores - 1: mobile always has the UI main thread (and RN's JS
        // thread) resident on the other cores — leave them one. Clamped
        // to >= 1 for single-core (or affinity-restricted) environments;
        // available_parallelism respects sched affinity on Android.
        let workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2)
            .saturating_sub(1)
            .max(1);
        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = std::sync::Arc::new(std::sync::Mutex::new(receiver));
        for i in 0..workers {
            let receiver = receiver.clone();
            std::thread::Builder::new()
                .name(format!("jsi-turbo-worker-{i}"))
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
