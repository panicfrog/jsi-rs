//! Scheduling of work back onto the JS thread.
//!
//! Async module methods run their futures on the shared [`pool`]; when they
//! finish, the resolve/reject call must happen on the JS thread (Hermes is
//! not thread-safe). In production the platform `CallInvoker` performs the
//! hop; `Inline` runs everything synchronously on the calling thread and
//! exists for single-threaded test harnesses.

use jsi::{CallInvoker, RuntimeHandle};

#[derive(Clone)]
pub enum Dispatcher {
    /// Schedules closures onto the JS thread via React Native's
    /// CallInvoker. `runtime` is the raw runtime pointer used to rebuild a
    /// [`RuntimeHandle`] on the JS thread (the invoker guarantees the
    /// callback runs on the thread that owns the runtime).
    CallInvoker {
        invoker: CallInvoker<'static>,
        runtime: *mut jsi::sys::Runtime,
    },
    /// Runs async method bodies synchronously on the calling (JS) thread.
    /// Testing only — no thread hop happens.
    Inline,
}

// CallInvoker is safe to move across threads (it is internally synchronized
// by React Native); the raw runtime pointer is only ever dereferenced on the
// JS thread that the invoker schedules us onto.
unsafe impl Send for Dispatcher {}
unsafe impl Sync for Dispatcher {}

pub type JsThreadJob = Box<dyn FnOnce(&mut RuntimeHandle<'static>) -> anyhow::Result<()> + Send>;

impl Dispatcher {
    /// Builds a production dispatcher from the CallInvoker the platform
    /// bindings installer received.
    pub fn call_invoker(invoker: CallInvoker<'static>, runtime: *mut jsi::sys::Runtime) -> Self {
        Dispatcher::CallInvoker { invoker, runtime }
    }

    /// Runs `job` on the JS thread. For `Inline` dispatchers this is a no-op
    /// mistake — inline execution never hops threads — so the job is dropped
    /// with an error log.
    pub fn dispatch(&self, job: JsThreadJob) {
        match self {
            Dispatcher::CallInvoker { invoker, runtime } => {
                let rt = *runtime;
                invoker.invoke_async(Box::new(move || {
                    let mut rt = RuntimeHandle::new_unchecked(rt);
                    job(&mut rt)
                }));
            }
            Dispatcher::Inline => {
                // inline flows finish on the calling thread and never call
                // dispatch(); dropping the job keeps single-threadedness
                eprintln!("jsi-turbo: dispatch() called on an Inline dispatcher; job dropped");
            }
        }
    }
}
