//! Proc macros for jsi-turbo. See `jsi_turbo` for the runtime.

use proc_macro_error2::proc_macro_error;
use syn::{parse_macro_input, ItemImpl, LitStr};

mod emit;
mod parse;
mod ts_types;

/// Defines a Rust TurboModule: the annotated impl block is the module's spec.
///
/// Generates:
/// - the original impl block, unchanged
/// - a `TurboModuleObject` impl with a const method table and per-method
///   erased invokers
/// - an entry in the `jsi_turbo::TURBO_MODULES` linkme slice (auto-registration)
/// - a `<crate>/generated/rust-turbo/<Name>.ts` TypeScript binding file
///   (set `RUST_TURBO_NO_TS=1` to disable, `RUST_TURBO_TS_DIR` to relocate)
///
/// Methods must be `fn(&self, [rt: &mut RuntimeHandle,] args...) ->
/// anyhow::Result<T>`; the `rt` parameter is optional. `#[turbo(getter)]` /
/// `#[turbo(setter)]` expose methods as properties. Module types must be
/// `Send + Sync + 'static` and implement `Default` (custom constructors are
/// planned).
#[proc_macro_error]
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn RustTurboModule(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let module_name = parse_macro_input!(attr as LitStr).value();
    let impl_block = parse_macro_input!(item as ItemImpl);

    let parsed = match parse::parse_impl(&module_name, impl_block) {
        Some(p) => p,
        None => return proc_macro::TokenStream::new(),
    };

    // TS emission first so it happens even if the rust codegen below aborts
    emit::emit_ts(&parsed);

    proc_macro::TokenStream::from(emit::emit_rust(&parsed))
}
