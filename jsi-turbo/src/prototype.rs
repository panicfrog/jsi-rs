//! Prototype construction shared by the `RustTurbo` proxy (modules) and
//! `object::to_js` (nested objects): one non-writable data property per
//! method, one accessor descriptor per getter/setter pair. Properties are
//! installed with `Object.defineProperty` (RN 0.88 jsi.h has no C++ API for
//! it — jsi.cpp itself calls the JS function).

use std::collections::BTreeMap;

use anyhow::Context;
use jsi::{AsValue, JsiFn, JsiObject, JsiValue, PropName, RuntimeHandle};

use crate::dispatcher::Dispatcher;
use crate::holder::{MethodEntry, MethodKind, MethodTable, ModuleHolder};

/// Builds the prototype object for a method table: one non-writable data
/// property per method, one accessor descriptor per getter/setter pair.
/// `dispatcher` is captured by async method entries (sync entries ignore
/// it).
pub(crate) fn build<'rt>(
    table: &'static MethodTable,
    dispatcher: &Dispatcher,
    rt: &mut RuntimeHandle<'rt>,
) -> anyhow::Result<JsiObject<'rt>> {
    let proto = JsiObject::new(rt);

    let mut props: BTreeMap<&'static str, PropBuilder<'rt>> = BTreeMap::new();
    for entry in table.entries {
        let f = make_entry_fn(table, entry, dispatcher.clone(), rt);
        let b = props.entry(entry.js_name).or_default();
        match entry.kind {
            MethodKind::Method => b.method = Some(f),
            MethodKind::Getter => b.get = Some(f),
            MethodKind::Setter => b.set = Some(f),
        }
    }

    for (name, b) in props {
        let mut d = JsiObject::new(rt);
        if let Some(f) = b.method {
            d.set(PropName::new("value", rt), &f.as_value(rt), rt);
            d.set(PropName::new("writable", rt), &JsiValue::new_bool(false), rt);
        }
        if let Some(f) = b.get {
            d.set(PropName::new("get", rt), &f.as_value(rt), rt);
        }
        if let Some(f) = b.set {
            d.set(PropName::new("set", rt), &f.as_value(rt), rt);
        }
        d.set(PropName::new("enumerable", rt), &JsiValue::new_bool(true), rt);
        d.set(PropName::new("configurable", rt), &JsiValue::new_bool(false), rt);
        define_property(rt, &proto, name, &d)?;
    }

    Ok(proto)
}

#[derive(Default)]
struct PropBuilder<'rt> {
    method: Option<JsiFn<'rt>>,
    get: Option<JsiFn<'rt>>,
    set: Option<JsiFn<'rt>>,
}

/// Creates the host function installed on the prototype for one method
/// table entry. The closure resolves `this` through the object's Rust
/// native state on every call.
fn make_entry_fn<'rt>(
    table: &'static MethodTable,
    entry: &'static MethodEntry,
    dispatcher: Dispatcher,
    rt: &mut RuntimeHandle<'rt>,
) -> JsiFn<'rt> {
    let js_name = entry.js_name;
    let module_name = table.module_name;

    JsiFn::from_host_fn(
        &PropName::new(js_name, rt),
        entry.param_count,
        Box::new(move |this, args, rt| {
            let obj: JsiObject = this
                .try_into_js(rt)
                .ok_or_else(|| anyhow::anyhow!("cannot call `{}` — `this` is not an object", js_name))?;
            let state = obj.get_rust_native_state(rt).ok_or_else(|| {
                anyhow::anyhow!(
                    "cannot call `{}` on module `{}` — `this` has no Rust state\nsuggestions:\n- did you destructure the module? (`const {{ {js_name} }} = ...`)\n- was the module disposed?",
                    js_name,
                    module_name
                )
            })?;
            let holder = state
                .get::<ModuleHolder>()
                .ok_or_else(|| anyhow::anyhow!("`{}` called on an object that is not a RustTurbo module", js_name))?;
            (entry.invoke)(&holder, &dispatcher, rt, &args)
        }),
        rt,
    )
}

pub(crate) fn define_property<'rt>(
    rt: &mut RuntimeHandle<'rt>,
    target: &JsiObject<'rt>,
    name: &str,
    descriptor: &JsiObject<'rt>,
) -> anyhow::Result<()> {
    let object_ctor = rt.global().get(PropName::new("Object", rt), rt);
    let object_ctor: JsiObject = object_ctor
        .try_into_js(rt)
        .context("global `Object` is not an object")?;
    let dp = object_ctor.get(PropName::new("defineProperty", rt), rt);
    let dp: JsiFn = dp
        .try_into_js(rt)
        .context("Object.defineProperty is not a function")?;

    dp.call(
        vec![
            target.as_value(rt),
            JsiValue::new_string(name, rt),
            descriptor.as_value(rt),
        ],
        rt,
    )?;
    Ok(())
}
