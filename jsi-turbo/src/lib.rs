//! # jsi-turbo
//!
//! A Rust TurboModule framework for React Native, in the spirit of
//! [Nitro Modules](https://github.com/margelo/nitro) but with Rust as the
//! single spec source: the `#[RustTurboModule("Name")]` attribute macro
//! generates the native method table, auto-registers the module into a
//! linkme distributed slice, and writes TypeScript bindings at macro
//! expansion time.
//!
//! At runtime, `globalThis.RustTurbo.get(name)` lazily constructs a
//! registered module as a plain JS object whose prototype carries
//! pre-installed methods; the Rust instance is held in the object's
//! `jsi::NativeState` and dies with the JS GC.
//!
//! ```no_run
//! # use jsi_turbo::RustTurboModule;
//! # use jsi::RuntimeHandle;
//! # #[derive(Default)]
//! # pub struct MathUtils;
//! #[RustTurboModule("MathUtils")]
//! impl MathUtils {
//!     fn add(&self, _rt: &mut RuntimeHandle, a: f64, b: f64) -> anyhow::Result<f64> {
//!         Ok(a + b)
//!     }
//! }
//! ```

pub mod dispatcher;
pub mod holder;
pub mod pool;
pub mod promise;
pub mod registry;
mod proxy;

pub use dispatcher::Dispatcher;
pub use holder::{InvokeFn, MethodEntry, MethodKind, MethodTable, ModuleHolder, TurboModuleObject};
pub use proxy::install;
pub use registry::{lookup, register, registered_names, ModuleDescriptor, TURBO_MODULES};

// re-exported so `#[RustTurboModule]` expansions can reference
// `::jsi_turbo::linkme::distributed_slice` without module crates depending
// on linkme directly
pub use linkme;

pub use jsi_turbo_macros::{RustTurboModule, TurboStruct};
