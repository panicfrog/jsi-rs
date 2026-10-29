//! Handing nested `#[TurboObject]` values to JS.
//!
//! The same wrapping the `RustTurbo` proxy does for module singletons, but
//! for values Rust code constructs and returns: prototype from the method
//! table, the [`ModuleHolder`] as native state, external memory pressure,
//! and the debug `__type` marker. See `docs/nested-objects.md` for the
//! design and the v1 trade-offs (per-call prototype build, no JS identity
//! caching, sync-only methods).

use jsi::{AsValue, IntoValue, JsiObject, JsiValue, PropName, RuntimeHandle};

use crate::dispatcher::Dispatcher;
use crate::holder::{ModuleHolder, TurboModuleObject};
use crate::prototype;

/// Recovers the Rust instance behind a JS-wrapped `#[TurboObject]` — the
/// same `Arc`, by reference. Called by the macro-generated invoker for
/// `Arc<T>` parameters (not a `FromValue` impl: that would trip the
/// orphan rule for `Arc<LocalType>`).
pub fn from_js<'rt, T: TurboModuleObject>(
    value: &JsiValue<'rt>,
    rt: &mut RuntimeHandle<'rt>,
) -> Option<std::sync::Arc<T>> {
    let obj: JsiObject = jsi::FromValue::from_value(value, rt)?;
    let state = obj.get_rust_native_state(rt)?;
    let holder = state.get::<ModuleHolder>()?;
    holder.downcast::<T>()
}

/// Wraps a `#[TurboObject]` value as a JS object whose prototype carries
/// its methods. Called by the `IntoValue` impl the `#[TurboObject]` macro
/// generates.
pub fn to_js<'rt, T: TurboModuleObject>(
    value: T,
    rt: &mut RuntimeHandle<'rt>,
) -> anyhow::Result<JsiValue<'rt>> {
    let name = T::__turbo_method_table().rust_type;
    let holder = ModuleHolder::with_instance(value);
    let table = holder.table();

    // objects are sync-only, so the dispatcher is never used by their
    // method entries; Inline is a placeholder that satisfies the signature
    let proto = prototype::build(table, &Dispatcher::Inline, rt)?;

    let obj = JsiObject::create_with_prototype(&proto.as_value(rt), rt);
    let external_size = holder.external_size();
    obj.set_rust_native_state(holder, rt);
    if external_size > 0 {
        obj.set_external_memory_pressure(external_size, rt);
    }
    #[cfg(debug_assertions)]
    {
        let mut d = JsiObject::new(rt);
        d.set(
            PropName::new("value", rt),
            &JsiValue::new_string(&format!("RustTurboObject<{name}>"), rt),
            rt,
        );
        d.set(PropName::new("writable", rt), &JsiValue::new_bool(false), rt);
        d.set(PropName::new("enumerable", rt), &JsiValue::new_bool(true), rt);
        d.set(PropName::new("configurable", rt), &JsiValue::new_bool(true), rt);
        prototype::define_property(rt, &obj, "__type", &d)?;
    }
    Ok(obj.into_value(rt))
}
