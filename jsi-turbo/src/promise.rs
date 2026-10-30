//! Async method plumbing: run a future off the JS thread and resolve the
//! JS Promise on the JS thread.
//!
//! `spawn` is called from the erased invoker (JS thread): it converts all
//! arguments to owned data beforehand, clones the module `Arc`, creates the
//! JS Promise, then either runs the future inline (`Dispatcher::Inline`) or
//! submits it to the worker pool and hops back with the dispatcher. Every
//! JSI touch — result conversion, `resolve`/`reject` calls — happens on the
//! JS thread; the future itself must be pure Rust data.

use std::future::Future;
use std::panic::{self, AssertUnwindSafe};
use std::pin::Pin;
use std::task::{Context, Poll};

use jsi::{IntoValue, JsiFn, JsiObject, JsiValue, RuntimeHandle};

use crate::dispatcher::Dispatcher;
use crate::executor;

/// Creates a JS Promise that settles with the outcome of `task`.
///
/// `T` must be convertible to a JS value **on the JS thread** (i.e. owned
/// Rust data: numbers, strings, `Vec`s, structs — never `JsiValue`).
pub fn spawn<'rt, T, F, Fut>(
    rt: &mut RuntimeHandle<'rt>,
    dispatcher: &Dispatcher,
    task: F,
) -> anyhow::Result<JsiObject<'rt>>
where
    // data types implement IntoValue for every lifetime; the HRTB also
    // structurally excludes JSI handle types (whose impls are tied to one
    // lifetime), matching the macro's async signature rules
    T: for<'a> IntoValue<'a> + Send + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<T>> + Send + 'static,
{
    let dispatcher = dispatcher.clone();
    let promise = jsi::create_promise(
        move |resolve: JsiFn<'rt>, reject: JsiFn<'rt>, rt: &mut RuntimeHandle<'rt>| {
            match &dispatcher {
                Dispatcher::Inline => {
                    // single-threaded (test harness): run the body right here
                    // and settle synchronously (futures-lite's block_on parks
                    // instead of spinning — same executor semantics as the
                    // CallInvoker path, just on this thread)
                    let result = futures_lite::future::block_on(run_task(task));
                    if let Err(e) = settle(rt, result, resolve, reject) {
                        eprintln!("jsi-turbo: settling promise failed: {e:#}");
                    }
                }
                Dispatcher::CallInvoker { .. } => {
                    // SAFETY: `resolve`/`reject` are JS functions owned by
                    // this runtime; we move them onto the JS thread via the
                    // dispatcher (the only place they will ever be called
                    // from), so widening the lifetime marker to 'static
                    // cannot outlive the runtime in practice.
                    let resolve: JsiFn<'static> = unsafe { std::mem::transmute(resolve) };
                    let reject: JsiFn<'static> = unsafe { std::mem::transmute(reject) };
                    detach(&dispatcher, task, resolve, reject);
                }
            }
        },
        rt,
    );
    Ok(promise)
}

/// Runs the future on the global executor and settles the promise on the
/// JS thread via the dispatcher.
fn detach<T, F, Fut>(
    dispatcher: &Dispatcher,
    task: F,
    resolve: JsiFn<'static>,
    reject: JsiFn<'static>,
) where
    T: for<'a> IntoValue<'a> + Send + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<T>> + Send + 'static,
{
    let dispatcher = dispatcher.clone();
    executor::spawn(async move {
        let result = run_task(task).await;
        dispatcher.dispatch(Box::new(move |rt| {
            if let Err(e) = settle(rt, result, resolve, reject) {
                eprintln!("jsi-turbo: settling promise failed: {e:#}");
            }
            Ok(())
        }));
    });
}

/// Runs the task, converting panics (both while creating the future and
/// inside it) into errors so the promise rejects instead of hanging.
async fn run_task<T, F, Fut>(task: F) -> anyhow::Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<T>> + Send,
{
    let fut = panic::catch_unwind(AssertUnwindSafe(task))
        .map_err(|p| anyhow::anyhow!("async method body panicked: {}", panic_message(&p)))?;
    match CatchUnwind(AssertUnwindSafe(fut)).await {
        Ok(result) => result,
        Err(p) => Err(anyhow::anyhow!("async method body panicked: {}", panic_message(&p))),
    }
}

/// `catch_unwind` for futures: polls the inner future, catching a panic
/// at any await point (std has no Future equivalent).
struct CatchUnwind<F>(AssertUnwindSafe<F>);

impl<F: Future> Future for CatchUnwind<F> {
    type Output = Result<F::Output, Box<dyn std::any::Any + Send>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: projection — F is not structurally pinned by us; the
        // inner future is free to move (it was caught-unwind-safe anyway)
        let inner = unsafe { self.map_unchecked_mut(|s| &mut *s.0) };
        match panic::catch_unwind(AssertUnwindSafe(|| inner.poll(cx))) {
            Ok(poll) => poll.map(Ok),
            Err(p) => Poll::Ready(Err(p)),
        }
    }
}

/// Converts the result and calls resolve/reject. Must run on the JS thread.
fn settle<'rt, T: for<'a> IntoValue<'a>>(
    rt: &mut RuntimeHandle<'rt>,
    result: anyhow::Result<T>,
    resolve: JsiFn<'rt>,
    reject: JsiFn<'rt>,
) -> anyhow::Result<()> {
    match result {
        Ok(value) => {
            let arg: JsiValue<'rt> = value.into_value(rt);
            resolve.call([arg], rt)?;
        }
        Err(e) => {
            let err = jsi::js_error!(rt, "{}", e);
            let arg: JsiValue<'rt> = err.into_value(rt);
            reject.call([arg], rt)?;
        }
    }
    Ok(())
}

fn panic_message(p: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = p.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".into()
    }
}
