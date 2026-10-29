//! Per-app aggregate library for the jsi-turbo example.
//!
//! One artifact (`librust_app.a` on iOS, `librust_app.so` on Android) is
//! what the platform glue links; it contains every Rust module crate plus
//! the jsi-turbo runtime. To add a module: depend on its crate here and add
//! its force anchor to `FORCE_LINK` (required for iOS staticlib linking —
//! see jsi_turbo::registry).

use std::pin::Pin;

#[cxx::bridge]
mod bridge {
    unsafe extern "C++" {
        include!("wrapper.h");

        #[namespace = "facebook::jsi"]
        type Runtime = jsi_sys::Runtime;
        #[namespace = "facebook::react"]
        type CallInvoker = jsi_sys::CallInvoker;
    }

    extern "Rust" {
        /// Called by the RustTurbo carrier on every JS runtime creation
        /// (iOS: `installJSIBindingsWithRuntime:callInvoker:`, Android: the
        /// `getBindingsInstaller()` installer). Installs
        /// `globalThis.RustTurbo` plus any legacy host-object globals.
        fn rust_turbo_install(rt: Pin<&mut Runtime>, invoker: SharedPtr<CallInvoker>) -> bool;
    }
}

// keeps module crates' objects (and their linkme slice entries) alive
// through iOS staticlib dead-code elimination; harmless elsewhere
#[used]
static FORCE_LINK: [extern "C" fn(); 1] = [jsi_demo_module::__jsi_turbo_force_turbo_demo];

fn rust_turbo_install(rt: Pin<&mut jsi_sys::Runtime>, invoker: cxx::SharedPtr<jsi_sys::CallInvoker>) -> bool {
    let rt: &mut jsi_sys::Runtime = unsafe { Pin::into_inner_unchecked(rt) };
    let raw = rt as *mut jsi_sys::Runtime;

    let (mut rt, invoker) = jsi::init(rt, invoker);
    let dispatcher = jsi_turbo::Dispatcher::call_invoker(invoker, raw);
    match jsi_turbo::install(&mut rt, dispatcher) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("rust-app: failed to install RustTurbo: {e:#}");
            false
        }
    }
}
