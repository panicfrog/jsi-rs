//! Code generation: the Rust expansion (method table + invokers +
//! registration) and the TypeScript binding file.

use heck::ToLowerCamelCase;
use proc_macro2::Span;
use proc_macro_error2::emit_warning;
use quote::{format_ident, quote};
use syn::{Ident, ItemStruct};

use crate::parse::{Item, MethodKind, ParsedModule};

pub fn emit_rust(m: &ParsedModule) -> proc_macro2::TokenStream {
    let ParsedModule {
        module_name,
        self_ty,
        impl_block,
        items,
        type_sym,
        ctor,
    } = m;

    let table_ident = format_ident!("__TURBO_TABLE_{}", type_sym.to_uppercase());
    let entry_ident = format_ident!("__TURBO_MODULE_{}", type_sym.to_uppercase());
    let force_ident = format_ident!("__jsi_turbo_force_{}", type_sym);

    // `ctor = "Type::create"` overrides default construction
    let construct = match ctor {
        Some(path) => quote! { ::anyhow::Context::context(#path(), ::std::concat!(
            "custom constructor of module ", #module_name, " failed"
        )) },
        None => quote! { Ok(<Self as ::std::default::Default>::default()) },
    };

    // per-method erased invokers + table entries
    let mut invokers = Vec::new();
    let mut table_entries = Vec::new();

    for item in items {
        let invoker_ident = format_ident!("__turbo_invoke_{}_{}", type_sym, item.rust_name);

        invokers.push(if item.is_async {
            emit_async_invoker(self_ty, module_name, &invoker_ident, item)
        } else {
            emit_invoker(self_ty, module_name, &invoker_ident, item)
        });

        let js_name = &item.js_name;
        let (kind, param_count) = match item.kind {
            MethodKind::Method => (quote! { ::jsi_turbo::MethodKind::Method }, item.params.len()),
            MethodKind::Getter => (quote! { ::jsi_turbo::MethodKind::Getter }, 0),
            MethodKind::Setter => (quote! { ::jsi_turbo::MethodKind::Setter }, 1),
        };
        table_entries.push(quote! {
            ::jsi_turbo::MethodEntry {
                js_name: #js_name,
                kind: #kind,
                param_count: #param_count,
                invoke: #invoker_ident,
            }
        });
    }

    quote! {
        #impl_block

        #[doc(hidden)]
        #(#invokers)*

        #[doc(hidden)]
        pub static #table_ident: ::jsi_turbo::MethodTable = ::jsi_turbo::MethodTable {
            module_name: #module_name,
            rust_type: ::std::stringify!(#self_ty),
            entries: &[#(#table_entries),*],
        };

        impl ::jsi_turbo::TurboModuleObject for #self_ty {
            fn __turbo_construct() -> ::anyhow::Result<Self> {
                #construct
            }
            fn __turbo_method_table() -> &'static ::jsi_turbo::MethodTable {
                &#table_ident
            }
        }

        // auto-registration (works out of the box in Rust binaries, tests and
        // cdylibs; on iOS staticlib the force anchor below must be referenced
        // by the aggregate library — see jsi_turbo::registry docs)
        #[doc(hidden)]
        #[linkme::distributed_slice(::jsi_turbo::TURBO_MODULES)]
        pub static #entry_ident: ::jsi_turbo::ModuleDescriptor = ::jsi_turbo::ModuleDescriptor {
            name: #module_name,
            rust_type: ::std::stringify!(#self_ty),
            ctor: || ::jsi_turbo::ModuleHolder::new::<#self_ty>(),
        };

        #[doc(hidden)]
        #[no_mangle]
        pub extern "C" fn #force_ident() {}
    }
}

fn emit_invoker(
    self_ty: &syn::Type,
    module_name: &str,
    invoker_ident: &Ident,
    item: &Item,
) -> proc_macro2::TokenStream {
    let js_name = &item.js_name;
    let method_name = &item.rust_name;
    let param_count = item.params.len();

    // argument extraction
    let mut arg_stmts = Vec::new();
    for (idx, (name, ty)) in item.params.iter().enumerate() {
        arg_stmts.push(quote! {
            let #name: #ty = match __args.get(#idx) {
                Some(__v) => ::jsi::FromValue::from_value(__v, __rt).ok_or_else(|| {
                    ::anyhow::anyhow!(
                        "argument `{}` of `{}.{}` could not be converted to {}",
                        stringify!(#name),
                        #module_name,
                        #js_name,
                        ::std::any::type_name::<#ty>(),
                    )
                })?,
                None => ::anyhow::bail!(
                    "not enough arguments: `{}.{}` expects {} argument(s)",
                    #module_name,
                    #js_name,
                    #param_count,
                ),
            };
        });
    }

    let param_names: Vec<_> = item.params.iter().map(|(n, _)| quote! { #n }).collect();
    let rt_arg = if item.takes_rt {
        quote! { __rt, }
    } else {
        quote! {}
    };

    let ret = if item.ret_is_unit {
        quote! {
            let __r: () = <#self_ty>::#method_name(&*__this, #rt_arg #(#param_names),*)
                .map_err(|e| e.context(::std::concat!(#module_name, ".", #js_name, " failed")))?;
            ::jsi::IntoValue::into_value(__r, __rt)
        }
    } else {
        let ret_ty = &item.ret;
        quote! {
            let __r: #ret_ty = <#self_ty>::#method_name(&*__this, #rt_arg #(#param_names),*)
                .map_err(|e| e.context(::std::concat!(#module_name, ".", #js_name, " failed")))?;
            ::jsi::IntoValue::into_value(__r, __rt)
        }
    };

    quote! {
        #[doc(hidden)]
        pub fn #invoker_ident<'rt>(
            __holder: &::jsi_turbo::ModuleHolder,
            __dispatcher: &::jsi_turbo::Dispatcher,
            __rt: &mut ::jsi::RuntimeHandle<'rt>,
            __args: &[::jsi::JsiValue<'rt>],
        ) -> ::anyhow::Result<::jsi::JsiValue<'rt>> {
            let _ = __dispatcher; // sync methods don't hop threads
            let __this: ::std::sync::Arc<#self_ty> = __holder
                .downcast::<#self_ty>()
                .ok_or_else(|| ::anyhow::anyhow!(
                    "`{}.{}` called on a module instance of the wrong type",
                    #module_name,
                    #js_name,
                ))?;
            #(#arg_stmts)*
            let __value = { #ret };
            Ok(__value)
        }
    }
}

/// Async invoker: converts arguments to owned data, spawns the future via
/// `jsi_turbo::promise` and returns a JS Promise. The body runs off the JS
/// thread; resolve/reject hop back through the dispatcher.
#[allow(clippy::too_many_arguments)]
fn emit_async_invoker(
    self_ty: &syn::Type,
    module_name: &str,
    invoker_ident: &Ident,
    item: &Item,
) -> proc_macro2::TokenStream {
    let js_name = &item.js_name;
    let method_name = &item.rust_name;
    let param_count = item.params.len();

    let mut arg_stmts = Vec::new();
    for (idx, (name, ty)) in item.params.iter().enumerate() {
        arg_stmts.push(quote! {
            let #name: #ty = match __args.get(#idx) {
                Some(__v) => ::jsi::FromValue::from_value(__v, __rt).ok_or_else(|| {
                    ::anyhow::anyhow!(
                        "argument `{}` of `{}.{}` could not be converted to {}",
                        stringify!(#name),
                        #module_name,
                        #js_name,
                        ::std::any::type_name::<#ty>(),
                    )
                })?,
                None => ::anyhow::bail!(
                    "not enough arguments: `{}.{}` expects {} argument(s)",
                    #module_name,
                    #js_name,
                    #param_count,
                ),
            };
        });
    }

    let param_names: Vec<_> = item.params.iter().map(|(n, _)| quote! { #n }).collect();
    let ret_ty = &item.ret;

    quote! {
        #[doc(hidden)]
        pub fn #invoker_ident<'rt>(
            __holder: &::jsi_turbo::ModuleHolder,
            __dispatcher: &::jsi_turbo::Dispatcher,
            __rt: &mut ::jsi::RuntimeHandle<'rt>,
            __args: &[::jsi::JsiValue<'rt>],
        ) -> ::anyhow::Result<::jsi::JsiValue<'rt>> {
            let __this: ::std::sync::Arc<#self_ty> = __holder
                .downcast::<#self_ty>()
                .ok_or_else(|| ::anyhow::anyhow!(
                    "`{}.{}` called on a module instance of the wrong type",
                    #module_name,
                    #js_name,
                ))?;
            #(#arg_stmts)*
            let __promise = ::jsi_turbo::promise::spawn::<#ret_ty, _, _>(__rt, __dispatcher, move || {
                async move {
                    <#self_ty>::#method_name(&*__this, #(#param_names),*).await
                }
            })
            .map_err(|e| e.context(::std::concat!(#module_name, ".", #js_name, " failed")))?;
            Ok(::jsi::IntoValue::into_value(__promise, __rt))
        }
    }
}

// ---------------------------------------------------------------------------
// #[derive(TurboStruct)]

pub fn emit_struct(input: &ItemStruct) -> Result<proc_macro2::TokenStream, ()> {
    let syn::Fields::Named(fields) = &input.fields else {
        proc_macro_error2::abort!(input, "#[derive(TurboStruct)] requires named fields");
    };
    let name = &input.ident;

    let tokens = quote! {
        impl<'rt> ::jsi::FromValue<'rt> for #name {
            fn from_value(
                value: &::jsi::JsiValue<'rt>,
                rt: &mut ::jsi::RuntimeHandle<'rt>,
            ) -> Option<Self> {
                let value = rt.clone(value);
                use ::jsi::DeserializeValue as _;
                #name::deserialize_value(value, rt).ok()
            }
        }

        impl<'rt> ::jsi::IntoValue<'rt> for #name {
            fn into_value(self, rt: &mut ::jsi::RuntimeHandle<'rt>) -> ::jsi::JsiValue<'rt> {
                use ::jsi::SerializeValue as _;
                match self.serialize_value(rt) {
                    Ok(v) => v,
                    Err(e) => panic!("failed to serialize struct {}: {e}", ::std::stringify!(#name)),
                }
            }
        }
    };

    // TS interface
    let mut members = Vec::new();
    let mut imports = Vec::new();
    for f in &fields.named {
        let fname = f.ident.as_ref().unwrap().to_string().to_lower_camel_case();
        let ty = crate::ts_types::ts_type_of(&f.ty, &mut imports);
        members.push(format!("  {fname}: {ty};"));
    }

    let content = format!(
        "// @rust-turbo generated by #[derive(TurboStruct)] — DO NOT EDIT.\n{imports}\nexport interface {name} {{\n{members}\n}}\n",
        imports = if imports.is_empty() {
            String::new()
        } else {
            format!("{}\n", imports.join("\n"))
        },
        members = members.join("\n"),
    );

    if let Some(dir) = ts_dir() {
        let path = dir.join("structs").join(format!("{}.ts", name));
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                emit_warning!(proc_macro2::Span::call_site(), "jsi-turbo: could not create {}: {}", parent.display(), e);
                return Ok(tokens);
            }
        }
        match std::fs::read_to_string(&path) {
            Ok(existing) if existing == content => {}
            _ => {
                if let Err(e) = std::fs::write(&path, content) {
                    emit_warning!(proc_macro2::Span::call_site(), "jsi-turbo: could not write {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(tokens)
}

// ---------------------------------------------------------------------------
// TypeScript emission

pub fn emit_ts(m: &ParsedModule) {
    let content = ts_content(m);
    let Some(dir) = ts_dir() else {
        return;
    };
    let file_name = format!("{}.ts", sanitize_ident(&m.module_name));
    let path = dir.join(file_name);

    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            emit_warning!(Span::call_site(), "jsi-turbo: could not create {}: {}", parent.display(), e);
            return;
        }
    }
    // write-if-changed so cargo fingerprints / editors don't churn
    match std::fs::read_to_string(&path) {
        Ok(existing) if existing == content => return,
        _ => {}
    }
    if let Err(e) = std::fs::write(&path, content) {
        emit_warning!(Span::call_site(), "jsi-turbo: could not write {}: {}", path.display(), e);
    }
}

fn ts_dir() -> Option<std::path::PathBuf> {
    if std::env::var_os("RUST_TURBO_NO_TS").is_some() {
        return None;
    }
    if let Some(dir) = std::env::var_os("RUST_TURBO_TS_DIR") {
        return Some(dir.into());
    }
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")?;
    Some(std::path::PathBuf::from(manifest).join("generated").join("rust-turbo"))
}

fn sanitize_ident(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "module".into()
    } else {
        out
    }
}

fn capitalize(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => name.to_string(),
    }
}

fn ts_content(m: &ParsedModule) -> String {
    let name = sanitize_ident(&m.module_name);
    let interface_name = capitalize(&name);
    let factory_name = format!("get{}", &interface_name);

    let mut imports: Vec<String> = Vec::new();

    // merge getter/setter pairs on the same JS name
    let mut method_lines: Vec<String> = Vec::new();
    let mut prop_lines: std::collections::BTreeMap<String, (Option<String>, bool)> =
        std::collections::BTreeMap::new(); // js_name -> (ts type, has_setter)

    for item in &m.items {
        let js = &item.js_name;
        match item.kind {
            MethodKind::Method => {
                let params: Vec<String> = item
                    .params
                    .iter()
                    .map(|(n, t)| {
                        format!("{}: {}", n.to_string().to_lower_camel_case(), crate::ts_types::ts_type_of(t, &mut imports))
                    })
                    .collect();
                let ret = if item.ret_is_unit { "void".into() } else { crate::ts_types::ts_type_of(&item.ret, &mut imports) };
                let ret = if item.is_async { format!("Promise<{ret}>") } else { ret };
                method_lines.push(format!("  {}({}): {};", js, params.join(", "), ret));
            }
            MethodKind::Getter => {
                let ty = crate::ts_types::ts_type_of(&item.ret, &mut imports);
                prop_lines
                    .entry(js.clone())
                    .or_insert((Some(ty), false));
            }
            MethodKind::Setter => {
                let ty = crate::ts_types::ts_type_of(&item.params[0].1, &mut imports);
                prop_lines
                    .entry(js.clone())
                    .and_modify(|(_, has_set)| *has_set = true)
                    .or_insert((Some(ty), true));
            }
        }
    }

    let mut members = method_lines;
    for (js, (ty, has_setter)) in prop_lines {
        let ty = ty.unwrap_or_else(|| "any".into());
        if has_setter {
            members.push(format!("  {}: {};", js, ty));
        } else {
            members.push(format!("  readonly {}: {};", js, ty));
        }
    }

    let import_block = if imports.is_empty() {
        String::new()
    } else {
        format!("{}\n\n", imports.join("\n"))
    };

    format!(
        "// @rust-turbo generated by #[RustTurboModule(\"{mod}\")] — DO NOT EDIT.\n\
         // Calls into the Rust module `{mod}` via the global RustTurbo proxy.\n\n\
         {imports}\
         type RustTurboGlobal = {{ get(name: string): unknown }};\n\n\
         declare global {{\n  var RustTurbo: RustTurboGlobal | undefined;\n}}\n\n\
         function bootstrap(): RustTurboGlobal {{\n  \
         // eslint-disable-next-line @typescript-eslint/no-var-requires\n  \
         const rn = require('react-native') as {{ TurboModuleRegistry: {{ get(name: string): unknown }} }};\n  \
         rn.TurboModuleRegistry.get('RustTurbo'); // triggers the native bindings installer\n  \
         if (!globalThis.RustTurbo) {{\n    throw new Error('RustTurbo bindings failed to install');\n  }}\n  \
         return globalThis.RustTurbo;\n}}\n\n\
         export interface {iface} {{\n{members}\n}}\n\n\
         export function {factory}(): {iface} {{\n  \
         return (globalThis.RustTurbo ?? bootstrap()).get('{mod}') as {iface};\n}}\n",
        imports = import_block,
        mod = m.module_name,
        iface = interface_name,
        members = members.join("\n"),
        factory = factory_name,
    )
}
