//! Rust type → TypeScript type mapping (V1 surface).

use syn::Type;

/// Whether the type can appear as a method parameter (i.e. has a usable
/// `FromValue` path).
pub fn is_supported_param_type(ty: &Type) -> bool {
    check(ty, false)
}

/// Whether the type can appear as a method return value (i.e. has a usable
/// `IntoValue` path).
pub fn is_supported_return_type(ty: &Type) -> bool {
    check(ty, true)
}

fn check(ty: &Type, ret: bool) -> bool {
    match ty {
        Type::Path(p) => {
            let Some(seg) = p.path.segments.last() else { return false };
            match seg.ident.to_string().as_str() {
                "f64" | "bool" | "String" | "JsiValue" | "JsiObject" => true,
                "i32" | "u32" | "usize" | "isize" => true,
                "Vec" | "Option" => generic_inner(seg).map(|t| check(t, ret)).unwrap_or(false),
                // return-only conveniences
                "i8" | "i16" | "i64" | "u8" | "u16" | "u64" | "f32" | "JsiString" | "JsiSymbol" | "JsiFn" => ret,
                _ => false,
            }
        }
        _ => false,
    }
}

/// The TypeScript type for a Rust type, or an error description.
pub fn ts_type(ty: &Type) -> String {
    match ty {
        Type::Path(p) => {
            let seg = p.path.segments.last().unwrap();
            match seg.ident.to_string().as_str() {
                "f64" | "f32" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
                | "usize" | "isize" => "number".into(),
                "bool" => "boolean".into(),
                "String" | "JsiString" => "string".into(),
                "JsiValue" | "JsiObject" | "JsiFn" | "JsiSymbol" => "any".into(),
                "Vec" => {
                    let inner = generic_inner(seg).map(ts_type).unwrap_or_else(|| "any".into());
                    format!("{inner}[]")
                }
                "Option" => {
                    let inner = generic_inner(seg).map(ts_type).unwrap_or_else(|| "any".into());
                    format!("{inner} | null")
                }
                other => format!("/* unsupported: {other} */ any"),
            }
        }
        _ => "any".into(),
    }
}

/// Human-readable rendering for diagnostics.
pub fn render_type(ty: &Type) -> String {
    quote::ToTokens::to_token_stream(ty).to_string()
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
