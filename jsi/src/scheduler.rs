//! # Scheduling work on the JS thread
//!
//! A JSI runtime may only be used from the thread that owns it. Hosts provide
//! a way to post work back onto that thread; this module abstracts over that
//! so that jsi stays independent of any particular host. React Native's
//! `CallInvoker` implements [`JsScheduler`] in the `jsi-rn` crate; other hosts
//! (an embedded Hermes, a QuickJS event loop, ...) implement it themselves.

/// A unit of work to run on the JS thread.
pub type JsCallback<'rt> = Box<dyn FnOnce() -> anyhow::Result<()> + 'rt>;

/// Posts work onto the thread that owns a JSI runtime.
pub trait JsScheduler: Send + Sync {
    /// Runs `job` on the JS thread at some point in the future.
    fn invoke_async(&self, job: JsCallback<'static>);

    /// Runs `job` on the JS thread and blocks until it has finished. Hosts
    /// that cannot block for the JS thread don't need to support this.
    fn invoke_sync(&self, job: JsCallback<'static>) {
        let _ = job;
        unimplemented!("this JsScheduler does not support synchronous invocation")
    }
}
