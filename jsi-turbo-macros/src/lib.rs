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

/// Defines a nested TurboObject: like a module, but not a global singleton.
/// Module methods construct values of this type and return them to JS;
/// JS holds the object (its Rust instance lives in native state, dropped by
/// the GC) and can pass it back as an `Arc<Self>` parameter.
///
/// ```ignore
/// #[TurboObject]
/// impl Layer {
///     pub fn add_point(&self, x: f64, y: f64) -> anyhow::Result<()>
///     #[turbo(getter)]
///     pub fn get_opacity(&self) -> anyhow::Result<f64>
/// }
///
/// #[RustTurboModule("Canvas")]
/// impl Canvas {
///     pub fn create_layer(&self, rt: &mut RuntimeHandle, name: String) -> anyhow::Result<Layer>
///     pub fn count_points(&self, rt: &mut RuntimeHandle, layer: Arc<Layer>) -> anyhow::Result<f64>
/// }
/// ```
///
/// Generates the method table and invokers (shared with modules), an
/// `IntoValue` impl handing instances to JS and a `FromValue` impl for
/// `Arc<Self>` (same instance, by reference). Methods are sync-only (async
/// needs the dispatcher, which object wrapping cannot reach — see
/// `jsi-turbo/docs/nested-objects.md`). TypeScript interfaces are emitted
/// alongside TurboStruct ones in `generated/rust-turbo/structs/`.
#[proc_macro_error]
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn TurboObject(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    if !attr.is_empty() {
        let span = proc_macro2::TokenStream::from(attr).into_iter().next().unwrap().span();
        proc_macro_error2::abort!(span, "#[TurboObject] takes no arguments");
    }
    let impl_block = parse_macro_input!(item as ItemImpl);

    let type_name = crate::ts_types::render_type(&impl_block.self_ty);
    let parsed = match parse::parse_impl(&type_name, impl_block, None) {
        Some(p) => p,
        None => return proc_macro::TokenStream::new(),
    };
    for item in &parsed.items {
        if item.is_async {
            proc_macro_error2::abort!(
                item.rust_name,
                "#[TurboObject] methods cannot be async";
                note = "async methods need the dispatcher, which object wrapping cannot reach; return plain data instead"
            );
        }
    }

    emit::emit_object_ts(&parsed);
    proc_macro::TokenStream::from(emit::emit_object_rust(&parsed))
}
