//! React Native integration for jsi-rs.
//!
//! `jsi` itself only depends on `jsi.h` and works with any JSI host. This
//! crate adds the React Native specific parts: wrapping React Native's
//! `CallInvoker` (which schedules work onto the JS thread) and the runtime
//! setup entry point used by React Native modules.

use std::marker::PhantomData;

use jsi::{JsCallback, JsScheduler, RuntimeHandle};

mod ffi;
#[cfg(target_os = "android")]
mod android;

/// Raw FFI bindings to the React Native types used by this crate.
pub mod sys {
    pub use crate::ffi::{CallInvoker, CallInvoker_invokeAsync, CallInvoker_invokeSync};

    #[cfg(target_os = "android")]
    pub use crate::android::CallInvokerHolder;
}

/// Used to run JavaScript functions in a given runtime from Rust. Required when
/// trying to call a JavaScript function from a thread other than the JS thread.
#[derive(Clone)]
pub struct CallInvoker<'rt>(
    pub(crate) cxx::SharedPtr<ffi::CallInvoker>,
    pub(crate) PhantomData<&'rt ()>,
);

unsafe impl Send for CallInvoker<'_> {}
unsafe impl Sync for CallInvoker<'_> {}

impl<'rt> CallInvoker<'rt> {
    pub fn new(ptr: cxx::SharedPtr<ffi::CallInvoker>) -> Self {
        CallInvoker(ptr, PhantomData)
    }

    /// WARNING: currently crashes with message "Synchronous native -> JS calls are currently not supported"
    pub fn invoke_sync(&self, job: JsCallback<'rt>) {
        #[cfg(feature = "call-invoker-trace")]
        log::trace!("call invoker sync call with closure at {:p}", job);

        unsafe {
            ffi::CallInvoker_invokeSync(self.0.clone(), Box::into_raw(Box::new(job)) as *mut _)
        }
    }

    pub fn invoke_async(&self, job: JsCallback<'rt>) {
        #[cfg(feature = "call-invoker-trace")]
        log::trace!("call invoker async call with closure at {:p}", job);

        unsafe {
            ffi::CallInvoker_invokeAsync(self.0.clone(), Box::into_raw(Box::new(job)) as *mut _)
        }
    }
}

impl JsScheduler for CallInvoker<'_> {
    fn invoke_async(&self, job: JsCallback<'static>) {
        CallInvoker::invoke_async(self, job)
    }

    fn invoke_sync(&self, job: JsCallback<'static>) {
        CallInvoker::invoke_sync(self, job)
    }
}

/// Wraps the raw `jsi::Runtime` and `CallInvoker` pointers handed over by
/// React Native. See [`jsi::init`] for the safety requirements on the runtime.
pub fn init(
    rt: *mut jsi::sys::Runtime,
    call_invoker: cxx::SharedPtr<ffi::CallInvoker>,
) -> (RuntimeHandle<'static>, CallInvoker<'static>) {
    #[cfg(feature = "call-invoker-trace")]
    log::debug!("got JSI call invoker pointer: {:p}", call_invoker.as_ref().unwrap());

    (jsi::init(rt), CallInvoker::new(call_invoker))
}
