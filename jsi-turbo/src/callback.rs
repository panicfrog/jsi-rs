//! JS functions passed to module methods as event callbacks.
//!
//! A `TurboCallback` wraps a JS function so Rust can invoke it later — the
//! event-subscription pattern (`onTick(cb)`). The function handle and the
//! raw runtime pointer are only ever dereferenced on the JS thread: with a
//! `CallInvoker` dispatcher the call hops there first (Nitro's
//! `AsyncJSCallback` pattern); an `Inline` dispatcher means the caller is
//! already on the JS thread (single-threaded test harnesses).

use std::sync::Arc;

use jsi::{JsiFn, JsiValue, RuntimeHandle, SerializeValue as _};

use crate::dispatcher::Dispatcher;

/// The shared part: what dispatched closures need on the JS thread.
struct Inner {
    fn_: JsiFn<'static>,
    runtime: *mut jsi::sys::Runtime,
}

// see the module docs for the safety argument
unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

/// A JS function captured from a method parameter. `Send + Sync` so modules
/// can store it behind a `Mutex` and fire it from any thread; clones share
/// the same function handle.
pub struct TurboCallback {
    inner: Arc<Inner>,
    dispatcher: Dispatcher,
}

impl Clone for TurboCallback {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            dispatcher: self.dispatcher.clone(),
        }
    }
}

impl TurboCallback {
    #[doc(hidden)]
    /// Captures the JS function in `value`. Called by macro-generated
    /// invokers, which have the runtime handle and dispatcher at hand.
    pub fn __capture<'rt>(
        value: &JsiValue<'rt>,
        rt: &mut RuntimeHandle<'rt>,
        dispatcher: &Dispatcher,
    ) -> Option<Self> {
        // `JsiFn` is only usable with its own runtime; extending it to
        // 'static is sound here because every use goes through `runtime`
        // (below) on the JS thread, and the handle itself keeps the
        // function alive against the GC.
        let fn_ = value.try_into_js::<JsiFn>(rt)?;
        let fn_ = unsafe { std::mem::transmute::<JsiFn<'rt>, JsiFn<'static>>(fn_) };
        let runtime = unsafe { std::pin::Pin::into_inner_unchecked(rt.get_inner_mut()) as *mut _ };
        Some(TurboCallback {
            inner: Arc::new(Inner { fn_, runtime }),
            dispatcher: dispatcher.clone(),
        })
    }

    /// Calls the JS function with one argument (any serializable value).
    pub fn call<A: serde::Serialize + Send + 'static>(&self, arg: A) {
        self.invoke(Box::new(move |rt| {
            let value = arg
                .serialize_value(rt)
                .map_err(|e| anyhow::anyhow!("serializing callback argument: {e}"))?;
            Ok(vec![value])
        }));
    }

    /// Calls the JS function with no arguments.
    pub fn call_with_no_args(&self) {
        self.invoke(Box::new(|_| Ok(vec![])));
    }

    /// Runs `prepare` on the JS thread to build the argument list, then
    /// calls the function. Errors (serialization or a JS exception) are
    /// logged and dropped — event delivery is fire-and-forget, like a JS
    /// `dispatchEvent`.
    fn invoke(
        &self,
        prepare: Box<
            dyn FnOnce(&mut RuntimeHandle<'static>) -> anyhow::Result<Vec<JsiValue<'static>>> + Send,
        >,
    ) {
        match &self.dispatcher {
            Dispatcher::Inline => {
                // single-threaded harness: the caller is on the JS thread
                let mut rt = RuntimeHandle::new_unchecked(self.inner.runtime);
                Self::fire(&self.inner, &mut rt, prepare);
            }
            Dispatcher::CallInvoker { .. } => {
                let inner = Arc::clone(&self.inner);
                self.dispatcher.dispatch(Box::new(move |rt| {
                    Self::fire(&inner, rt, prepare);
                    Ok(())
                }));
            }
        }
    }

    fn fire(
        inner: &Inner,
        rt: &mut RuntimeHandle<'static>,
        prepare: Box<
            dyn FnOnce(&mut RuntimeHandle<'static>) -> anyhow::Result<Vec<JsiValue<'static>>> + Send,
        >,
    ) {
        if let Err(e) = Self::do_call(inner, rt, prepare) {
            eprintln!("jsi-turbo: TurboCallback invocation failed: {e:#}");
        }
    }

    fn do_call(
        inner: &Inner,
        rt: &mut RuntimeHandle<'static>,
        prepare: Box<
            dyn FnOnce(&mut RuntimeHandle<'static>) -> anyhow::Result<Vec<JsiValue<'static>>> + Send,
        >,
    ) -> anyhow::Result<()> {
        let args = prepare(rt)?;
        inner
            .fn_
            .call(args, rt)
            .map_err(|e| anyhow::anyhow!("JS callback threw an exception: {}", e.what()))?;
        Ok(())
    }
}
