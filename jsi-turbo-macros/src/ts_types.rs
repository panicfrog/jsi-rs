//! Rust type ↔ TypeScript type mapping.
//!
//! Known data types map to TS primitives; JSI handle types are sync-only;
//! any other named type is assumed to be a `#[derive(TurboStruct)]` struct
//! and is emitted as an `import type` from `./structs/` (the Rust compiler
//! enforces the actual `FromValue`/`IntoValue` bounds at call sites).

use syn::Type;

/// Whether the type is a JSI handle (`JsiValue`, `JsiObject`, ...) that is
/// only usable in sync methods.
pub fn is_jsi_handle_type(ty: &Type) -> bool {
    match last_segment(ty) {
        Some(seg) => matches!(
            seg.ident.to_string().as_str(),
            "JsiValue" | "JsiObject" | "JsiString" | "JsiSymbol" | "JsiFn" | "JsiArrayBuffer"
        ),
        None => false,
    }
}

/// Whether the type is `TurboCallback` (a JS function captured as an event
/// callback; handled specially by the invokers).
pub fn is_turbo_callback(ty: &Type) -> bool {
    matches!(last_segment(ty), Some(seg) if seg.ident == "TurboCallback")
}

/// The TypeScript type for a Rust type. Unknown named types are treated as
/// TurboStruct references and add an import line to `imports`.
pub fn ts_type_of(ty: &Type, imports: &mut Vec<String>) -> String {
    let Some(seg) = last_segment(ty) else {
        return "any".into();
    };
    match seg.ident.to_string().as_str() {
        "f64" | "f32" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
        | "usize" | "isize" => "number".into(),
        "bool" => "boolean".into(),
        "String" | "JsiString" => "string".into(),
        "JsiValue" | "JsiObject" | "JsiFn" | "JsiSymbol" => "any".into(),
        "JsiArrayBuffer" => "ArrayBuffer".into(),
        "Vec" => {
            let inner = generic_inner(seg)
                .map(|t| ts_type_of(t, imports))
                .unwrap_or_else(|| "any".into());
            format!("{inner}[]")
        }
        "Option" => {
            let inner = generic_inner(seg)
                .map(|t| ts_type_of(t, imports))
                .unwrap_or_else(|| "any".into());
            format!("{inner} | null")
        }
        "TurboCallback" => {
            // a JS function Rust can call later (event subscription)
            let inner = generic_inner(seg)
                .map(|t| ts_type_of(t, imports))
                .unwrap_or_else(|| "any".into());
            format!("(arg: {inner}) => void")
        }
        other => {
            // assumed #[derive(TurboStruct)] struct
            let import = format!("import type {{ {other} }} from './structs/{other}';");
            if !imports.contains(&import) {
                imports.push(import);
            }
            other.into()
        }
    }
}

/// Human-readable rendering for diagnostics.
pub fn render_type(ty: &Type) -> String {
    quote::ToTokens::to_token_stream(ty).to_string()
}

fn last_segment(ty: &Type) -> Option<&syn::PathSegment> {
    match ty {
        Type::Path(p) => p.path.segments.last(),
        _ => None,
    }
}

fn generic_inner(seg: &syn::PathSegment) -> Option<&Type> {
    let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
        return None;
    };
    match args.args.first() {
        Some(syn::GenericArgument::Type(t)) => Some(t),
        _ => None,
    }
}
