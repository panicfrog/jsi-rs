//! Parsing of `#[RustTurboModule("Name")]` impl blocks.

use heck::ToLowerCamelCase;
use proc_macro_error2::{abort, emit_error};
use syn::{FnArg, Ident, ImplItemFn, ItemImpl, Pat, ReturnType, Type};

pub enum MethodKind {
    Method,
    Getter,
    Setter,
}

pub struct Item {
    /// Rust method name
    pub rust_name: Ident,
    /// Name JavaScript sees
    pub js_name: String,
    pub kind: MethodKind,
    /// Whether this is an `async fn` (exposed to JS as a Promise)
    pub is_async: bool,
    /// User parameters (excluding `&self` and the optional runtime handle)
    pub params: Vec<(Ident, Type)>,
    /// Whether the method takes a `&mut RuntimeHandle` parameter
    pub takes_rt: bool,
    /// The success type `T` of `-> anyhow::Result<T>`
    pub ret: Type,
    /// Whether the success type is `()`
    pub ret_is_unit: bool,
}

pub struct ParsedModule {
    /// The module name from the attribute (used for registration and TS)
    pub module_name: String,
    /// The implementing type
    pub self_ty: Type,
    /// The impl block, emitted unchanged
    pub impl_block: ItemImpl,
    pub items: Vec<Item>,
    /// Identifier-safe version of the type name for generated symbols
    pub type_sym: String,
}

pub fn parse_impl(module_name: &str, mut impl_block: ItemImpl) -> Option<ParsedModule> {
    if impl_block.trait_.is_some() {
        abort!(impl_block.self_ty, "#[RustTurboModule] cannot be used on trait impls");
    }

    let self_ty = (*impl_block.self_ty).clone();
    let type_sym = type_symbol(&self_ty);

    let mut items = Vec::new();
    // (js_name, kind) pairs; a getter+setter pair on one name is legal
    let mut seen_js_names: Vec<(String, MethodKind)> = Vec::new();

    for item in &mut impl_block.items {
        let syn::ImplItem::Fn(f) = item else {
            emit_error!(
                item,
                "#[RustTurboModule] impl blocks should only contain methods";
                note = "put other items in a separate impl block"
            );
            continue;
        };
        match parse_method(f) {
            Some(it) => {
                let conflict = seen_js_names
                    .iter()
                    .find(|(n, kind)| *n == it.js_name && !is_legal_pair(kind, &it.kind));
                if let Some((name, _)) = conflict {
                    emit_error!(
                        f.sig.ident,
                        "duplicate JS name `{}` in module `{}`", name, module_name;
                        note = "only one getter + one setter may share a name; use #[turbo(method as otherName)] to disambiguate"
                    );
                } else {
                    seen_js_names.push((it.js_name.clone(), match it.kind {
                        MethodKind::Method => MethodKind::Method,
                        MethodKind::Getter => MethodKind::Getter,
                        MethodKind::Setter => MethodKind::Setter,
                    }));
                }
                items.push(it);
            }
            None => continue,
        }
    }

    if items.is_empty() {
        emit_error!(
            impl_block.self_ty,
            "#[RustTurboModule] impl block has no exposed methods"
        );
        return None;
    }

    Some(ParsedModule {
        module_name: module_name.to_string(),
        self_ty,
        impl_block,
        items,
        type_sym,
    })
}

/// Parses the `#[turbo(...)]` helper attribute: `getter` / `setter` /
/// `method`, each optionally `as explicitName`.
enum Helper {
    Method(Option<Ident>),
    Getter(Option<Ident>),
    Setter(Option<Ident>),
}

struct TurboHelper(Helper);

impl syn::parse::Parse for TurboHelper {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        let make: fn(Option<Ident>) -> Helper = match ident.to_string().as_str() {
            "getter" => Helper::Getter,
            "setter" => Helper::Setter,
            "method" => Helper::Method,
            other => {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("unknown turbo helper `{other}`, expected `getter`, `setter` or `method`"),
                ))
            }
        };
        let name = if input.peek(syn::Token![as]) {
            input.parse::<syn::Token![as]>()?;
            Some(input.parse::<Ident>()?)
        } else {
            None
        };
        Ok(TurboHelper(make(name)))
    }
}

fn parse_method(f: &mut ImplItemFn) -> Option<Item> {
    let is_async = f.sig.asyncness.is_some();

    // helper attribute
    let mut helper: Option<Helper> = None;
    f.attrs.retain(|attr| {
        if !attr.path().is_ident("turbo") {
            return true;
        }
        match attr.parse_args::<TurboHelper>() {
            Ok(h) => helper = Some(h.0),
            Err(e) => emit_error!(attr, "{}", e),
        }
        false
    });

    let helper = helper.unwrap_or(Helper::Method(None));

    // receiver
    let receiver_ok = matches!(&f.sig.inputs[0], FnArg::Receiver(r) if r.reference.is_some() && r.mutability.is_none());
    if f.sig.inputs.is_empty() || !receiver_ok {
        emit_error!(
            f.sig.inputs,
            "TurboModule methods must take `&self` as their first parameter"
        );
        return None;
    }

    // parameters: an optional `rt: &mut RuntimeHandle` first, then user params
    let mut takes_rt = false;
    let mut params = Vec::new();
    for arg in f.sig.inputs.iter().skip(1) {
        let FnArg::Typed(pt) = arg else {
            emit_error!(arg, "unexpected receiver argument");
            return None;
        };
        let is_rt = type_is_runtime_handle(&pt.ty);
        if is_rt {
            if is_async {
                emit_error!(
                    pt,
                    "async methods cannot take a RuntimeHandle";
                    note = "the body runs on a worker thread and must not touch JSI; convert values before awaiting"
                );
                return None;
            }
            if !takes_rt && params.is_empty() {
                takes_rt = true;
                continue;
            } else {
                emit_error!(pt, "the runtime handle must be the first parameter after `&self`");
                return None;
            }
        }
        let name = match &*pt.pat {
            Pat::Ident(pi) => pi.ident.clone(),
            other => {
                emit_error!(other, "parameters must be plain identifiers");
                return None;
            }
        };
        let is_jsi = crate::ts_types::is_jsi_handle_type(&pt.ty);
        if is_jsi && is_async {
            emit_error!(
                pt,
                "async methods cannot take JSI handle types (`{}`)", crate::ts_types::render_type(&pt.ty);
                note = "the body runs on a worker thread and must only touch owned Rust data"
            );
            return None;
        }
        params.push((name, (*pt.ty).clone()));
    }

    // return type: anyhow::Result<T>
    let (ret, ret_is_unit) = match &f.sig.output {
        ReturnType::Default => {
            emit_error!(
                f.sig.output,
                "TurboModule methods must return `anyhow::Result<T>`"
            );
            return None;
        }
        ReturnType::Type(_, ty) => match result_inner(ty) {
            Some(inner) => {
                let is_unit = type_is_unit(inner);
                if !is_unit && is_async && crate::ts_types::is_jsi_handle_type(inner) {
                    emit_error!(
                        ty,
                        "async methods cannot return JSI handle types (`{}`)", crate::ts_types::render_type(inner);
                        note = "convert to owned Rust data; the value crosses back on the JS thread"
                    );
                    return None;
                }
                (inner.clone(), is_unit)
            }
            None => {
                emit_error!(
                    ty,
                    "TurboModule methods must return `anyhow::Result<T>`"
                );
                return None;
            }
        },
    };

    // shape checks per kind
    match &helper {
        Helper::Getter(_) => {
            if is_async {
                emit_error!(f.sig.ident, "getters cannot be async");
                return None;
            }
            if !params.is_empty() {
                emit_error!(f.sig.inputs, "getters cannot take parameters (besides the runtime handle)");
                return None;
            }
        }
        Helper::Setter(_) => {
            if is_async {
                emit_error!(f.sig.ident, "setters cannot be async");
                return None;
            }
            if params.len() != 1 {
                emit_error!(f.sig.inputs, "setters take exactly one value parameter");
                return None;
            }
            if !ret_is_unit {
                emit_error!(f.sig.inputs, "setters must return `anyhow::Result<()>`");
                return None;
            }
        }
        Helper::Method(_) => {}
    }

    let (kind, js_name) = match helper {
        Helper::Method(name) => {
            let js = name.map(|i| i.to_string()).unwrap_or_else(|| f.sig.ident.to_string().to_lower_camel_case());
            (MethodKind::Method, js)
        }
        Helper::Getter(name) => {
            let base = name.map(|i| i.to_string()).unwrap_or_else(|| f.sig.ident.to_string());
            let base = base.strip_prefix("get_").unwrap_or(&base).to_string();
            (MethodKind::Getter, base.to_lower_camel_case())
        }
        Helper::Setter(name) => {
            let base = name.map(|i| i.to_string()).unwrap_or_else(|| f.sig.ident.to_string());
            let base = base.strip_prefix("set_").unwrap_or(&base).to_string();
            (MethodKind::Setter, base.to_lower_camel_case())
        }
    };

    Some(Item {
        rust_name: f.sig.ident.clone(),
        js_name,
        kind,
        is_async,
        params,
        takes_rt,
        ret,
        ret_is_unit,
    })
}

fn is_legal_pair(a: &MethodKind, b: &MethodKind) -> bool {
    matches!(
        (a, b),
        (MethodKind::Getter, MethodKind::Setter) | (MethodKind::Setter, MethodKind::Getter)
    )
}

// ---------------------------------------------------------------------------
// small type helpers

fn type_symbol(ty: &Type) -> String {
    use heck::ToSnakeCase;
    let rendered = quote::ToTokens::to_token_stream(ty).to_string();
    let mut sym = String::new();
    for c in rendered.chars() {
        if c.is_ascii_alphanumeric() {
            sym.push(c);
        } else if !sym.ends_with('_') {
            sym.push('_');
        }
    }
    sym.trim_matches('_').to_snake_case()
}

fn last_segment(ty: &Type) -> Option<&syn::PathSegment> {
    match ty {
        Type::Path(p) => p.path.segments.last(),
        _ => None,
    }
}

fn type_is_runtime_handle(ty: &Type) -> bool {
    match ty {
        Type::Reference(r) => r.mutability.is_some() && last_segment(&r.elem).map(|s| s.ident == "RuntimeHandle").unwrap_or(false),
        _ => false,
    }
}

fn type_is_unit(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(t) if t.elems.is_empty())
}

fn result_inner(ty: &Type) -> Option<&Type> {
    let seg = last_segment(ty)?;
    if seg.ident != "Result" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
        return None;
    };
    let first = args.args.first()?;
    Some(match first {
        syn::GenericArgument::Type(t) => t,
        _ => return None,
    })
}
