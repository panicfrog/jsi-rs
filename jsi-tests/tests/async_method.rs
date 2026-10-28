//! Regression tests for `#[host_object]` async methods.
//!
//! Async methods used to generate uncompilable code: the macro called
//! `js_error!(err, rt)` with swapped arguments, moved the `rt` handle into
//! the method call and then used it again, and required the task callback to
//! be `'static`. These tests compile the async path (the compile itself is
//! the regression assertion) and then drive it end-to-end on a real Hermes
//! runtime: the resolve path through `Ok`, the reject path through `Err`
//! (which runs `js_error!` at runtime).

mod common;

use std::cell::RefCell;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use common::create_raw_runtime;
use jsi::{
    host_object, AsyncUserHostObject, IntoValue, JsTaskCallback, JsiValue, PropName,
    RuntimeHandle,
};

fn with_rt(f: impl FnOnce(&mut RuntimeHandle<'static>, &mut cxx::UniquePtr<jsi_sys::Runtime>)) {
    let mut raw = create_raw_runtime();
    let ptr = unsafe { raw.as_mut().unwrap().get_unchecked_mut() as *mut _ };
    let (mut rt, _invoker) = jsi::init(ptr, cxx::SharedPtr::null());
    f(&mut rt, &mut raw);
}

fn eval<'rt>(
    _rt: &mut RuntimeHandle<'rt>,
    raw: &mut cxx::UniquePtr<jsi_sys::Runtime>,
    js: &str,
) -> JsiValue<'rt> {
    JsiValue::from_raw(jsi_tests::ffi::bridge::eval_js(raw.pin_mut(), js))
}

// A minimal executor for the spawned tasks: spawn() stashes them here, the
// test drives them on the JS thread (where the runtime lives). Thread-local
// so that tests running in parallel don't steal each other's tasks.
thread_local! {
    static TASKS: RefCell<Vec<JsTaskCallback<'static>>> = RefCell::new(Vec::new());
}

fn take_tasks() -> Vec<JsTaskCallback<'static>> {
    TASKS.with(|t| std::mem::take(&mut *t.borrow_mut()))
}

// A busy-poll block_on; the futures spawned by the host_object macro have no
// real suspension points, so a single poll runs them to completion.
fn block_on<F: Future>(fut: F) -> F::Output {
    unsafe fn noop(_: *const ()) {}
    unsafe fn clone(p: *const ()) -> RawWaker {
        RawWaker::new(p, &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut cx = Context::from_waker(&waker);
    let mut fut = pin!(fut);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(out) => return out,
            Poll::Pending => std::hint::spin_loop(),
        }
    }
}

// Runs every spawned task against the given runtime. The task must be called
// with a handle borrowed for as long as the future lives, so the handle is
// leaked to satisfy `'static` (one leak per test; the process is short-lived).
fn drive_tasks(raw: &mut cxx::UniquePtr<jsi_sys::Runtime>) {
    for task in take_tasks() {
        let ptr = unsafe { raw.as_mut().unwrap().get_unchecked_mut() as *mut _ };
        let rt: &'static mut RuntimeHandle<'static> =
            Box::leak(Box::new(RuntimeHandle::new_unchecked(ptr)));
        if let Err(e) = block_on(task(rt)) {
            panic!("spawned task failed: {e:?}");
        }
    }
}

struct AsyncDemo;

#[host_object]
impl AsyncDemo {
    pub async fn slow_add<'rt>(
        &self,
        _rt: &mut RuntimeHandle<'rt>,
        a: f64,
        b: f64,
    ) -> anyhow::Result<f64> {
        Ok(a + b)
    }

    // Always fails; the rejection path is what exercises js_error!.
    pub async fn fail<'rt>(&self, _rt: &mut RuntimeHandle<'rt>) -> anyhow::Result<f64> {
        anyhow::bail!("deliberate failure")
    }
}

impl AsyncUserHostObject for AsyncDemo {
    fn spawn<'rt>(task: JsTaskCallback<'rt>) {
        // The tests only ever spawn tasks while the runtime is borrowed for
        // 'static (see with_rt), so upgrading the callback is sound here.
        let task: JsTaskCallback<'static> = unsafe { std::mem::transmute(task) };
        TASKS.with(|t| t.borrow_mut().push(task));
    }

    fn invoke(cb: jsi::CallInvokerCallback) {
        let _ = cb();
    }
}

#[test]
fn host_object_async_method_resolves() {
    with_rt(|rt, raw| {
        rt.global()
            .set(PropName::new("AsyncDemo", rt), &AsyncDemo.into_value(rt), rt);

        let promise = eval(
            rt,
            raw,
            "globalThis.result = null; AsyncDemo.slowAdd(19, 23).then(v => { globalThis.result = v; });",
        );
        assert!(promise.is_object(), "async method should return a promise");

        drive_tasks(raw);
        raw.pin_mut().drain_microtasks(-1);

        let out = eval(rt, raw, "globalThis.result");
        assert_eq!(42., out.into_js::<f64>(rt));
    });
}

#[test]
fn host_object_async_method_rejects() {
    with_rt(|rt, raw| {
        rt.global()
            .set(PropName::new("AsyncDemo", rt), &AsyncDemo.into_value(rt), rt);

        let promise = eval(
            rt,
            raw,
            "globalThis.err = null; AsyncDemo.fail().catch(e => { globalThis.err = e && e.message; });",
        );
        assert!(promise.is_object(), "async method should return a promise");

        drive_tasks(raw);
        raw.pin_mut().drain_microtasks(-1);

        let out = eval(
            rt,
            raw,
            "typeof globalThis.err === 'string' ? globalThis.err : String(globalThis.err)",
        );
        let msg: String = out.into_js(rt);
        assert!(
            msg.contains("deliberate failure"),
            "rejection should carry the error message, got: {msg:?}"
        );
    });
}
