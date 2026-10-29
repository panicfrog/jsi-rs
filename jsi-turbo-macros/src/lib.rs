//! Proc macros for jsi-turbo. See `jsi_turbo` for the runtime.

use proc_macro_error2::proc_macro_error;
use syn::{parse_macro_input, ItemImpl, ItemStruct, LitStr};

/// The `#[RustTurboModule(...)]` attribute: `#[RustTurboModule("Name")]` or
/// `#[RustTurboModule("Name", ctor = "Type::create")]`.
struct ModuleAttr {
    name: LitStr,
    ctor: Option<LitStr>,
}

impl syn::parse::Parse for ModuleAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: LitStr = input.parse()?;
        let mut ctor = None;
        while input.peek(syn::Token![,]) {
            input.parse::<syn::Token![,]>()?;
            let ident: syn::Ident = input.parse()?;
            match ident.to_string().as_str() {
                "ctor" => {
                    input.parse::<syn::Token![=]>()?;
                    let path: LitStr = input.parse()?;
                    if ctor.replace(path).is_some() {
                        return Err(syn::Error::new(ident.span(), "duplicate `ctor` option"));
                    }
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown option `{other}`, expected `ctor = \"path\"`"),
                    ))
                }
            }
        }
        Ok(ModuleAttr { name, ctor })
    }
}

mod emit;
mod parse;
mod ts_types;

/// Derives JSI value conversions (via serde) and emits a TypeScript
/// interface for a struct, so it can cross the TurboModule boundary as a
/// typed parameter/return value:
///
/// ```no_run
/// # use serde::{Serialize, Deserialize};
/// # use jsi_turbo::TurboStruct;
/// #[derive(Serialize, Deserialize, TurboStruct)]
/// pub struct Point { pub x: f64, pub y: f64 }
/// ```
///
/// Requires `Serialize` + `Deserialize` (from serde) on the same struct.
/// Generates `impl FromValue` / `impl IntoValue` and writes
/// `<crate>/generated/rust-turbo/structs/<Name>.ts`.
#[proc_macro_error]
#[proc_macro_derive(TurboStruct)]
pub fn turbo_struct(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    match emit::emit_struct(&input) {
        Ok(tokens) => proc_macro::TokenStream::from(tokens),
        Err(()) => proc_macro::TokenStream::new(),
    }
}

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
/// `Send + Sync + 'static` and implement `Default`, or supply a custom
/// constructor via `#[RustTurboModule("Name", ctor = "Type::create")]`
/// (any `fn() -> anyhow::Result<Type>` path works).
#[proc_macro_error]
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn RustTurboModule(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let attr = parse_macro_input!(attr as ModuleAttr);
    let impl_block = parse_macro_input!(item as ItemImpl);

    let ctor = attr.ctor.as_ref().map(|lit| {
        syn::parse_str::<syn::Path>(&lit.value()).unwrap_or_else(|e| {
            proc_macro_error2::abort!(lit, "invalid `ctor` path `{}`: {e}", lit.value())
        })
    });

    let parsed = match parse::parse_impl(&attr.name.value(), impl_block, ctor) {
        Some(p) => p,
        None => return proc_macro::TokenStream::new(),
    };

    // TS emission first so it happens even if the rust codegen below aborts
    emit::emit_ts(&parsed);

    proc_macro::TokenStream::from(emit::emit_rust(&parsed))
}
