//! The per-runtime `RustTurbo` proxy installed as a JS global.
//!
//! `globalThis.RustTurbo.get(name)` lazily constructs a registered module and
//! returns a plain JS object whose prototype carries pre-installed methods
//! (Nitro-style): the object itself holds the Rust [`ModuleHolder`] as
//! `jsi::NativeState`, and every method call resolves `this` through that
//! state. All per-runtime caches live in the `get` host-function's closure,
//! so they die with the runtime.

use std::collections::{BTreeMap, HashMap};

use anyhow::{bail, Context};
use jsi::{AsValue, IntoValue, JsiFn, JsiObject, JsiValue, PropName, RuntimeHandle};

use crate::dispatcher::Dispatcher;
use crate::holder::{MethodEntry, MethodKind, MethodTable, ModuleHolder};
use crate::registry;

struct ProxyState {
    /// how async work hops back to the JS thread
    dispatcher: Dispatcher,
    /// prototypes keyed by MethodTable address
    prototypes: HashMap<usize, JsiObject<'static>>,
    /// constructed module instances keyed by name (lazy singletons)
    instances: HashMap<String, JsiObject<'static>>,
}

/// Installs `globalThis.RustTurbo = { get(name) }` into the runtime.
/// Call once per runtime (e.g. from the platform carrier's bindings
/// installer, before the JS bundle runs).
///
/// Pass [`Dispatcher::call_invoker`] in production; `Dispatcher::Inline`
/// runs async method bodies synchronously and is meant for single-threaded
/// test harnesses.
pub fn install(rt: &mut RuntimeHandle<'static>, dispatcher: Dispatcher) -> anyhow::Result<()> {
    // surface duplicate registrations (two crates claiming the same module
    // name) as early as possible instead of at first `get`
    let duplicates = registry::duplicate_names();
    debug_assert!(
        duplicates.is_empty(),
        "duplicate RustTurbo module names: {}",
        duplicates.join("; ")
    );

    let mut proxy = JsiObject::new(rt);
    let mut state = ProxyState {
        dispatcher,
        prototypes: HashMap::new(),
        instances: HashMap::new(),
    };

    let get = JsiFn::from_host_fn(
        &PropName::new("get", rt),
        1,
        Box::new(move |_this, mut args, rt| {
            let name: String = match args.len() {
                1 => args
                    .remove(0)
                    .try_into_js(rt)
                    .context("RustTurbo.get expects a module name string")?,
                _ => bail!("RustTurbo.get expects exactly one argument (the module name)"),
            };

            // cached instance?
            if let Some(obj) = state.instances.get(&name) {
                let obj: JsiObject = rt.clone(obj);
                return Ok(obj.into_value(rt));
            }

            let descriptor = registry::lookup(&name).ok_or_else(|| {
                anyhow::anyhow!(
                    "no RustTurbo module named `{}` is registered (registered: {})",
                    name,
                    registry::registered_names().join(", ")
                )
            })?;
            let holder = (descriptor.ctor)()
                .with_context(|| format!("failed to construct RustTurbo module `{}`", name))?;
            let table = holder.table();

            // prototype for this module type, built once per runtime
            let proto = match state.prototypes.get(&(table as *const _ as usize)) {
                Some(p) => rt.clone(p),
                None => {
                    let dispatcher = state.dispatcher.clone();
                    let p = build_prototype(table, &dispatcher, rt)?;
                    state
                        .prototypes
                        .insert(table as *const _ as usize, rt.clone(&p));
                    p
                }
            };

            let obj = JsiObject::create_with_prototype(&proto.as_value(rt), rt);
            // report native memory to the JS GC before the holder becomes
            // unreachable from Rust (Nitro does the same when handing a
            // hybrid object to JS)
            let external_size = holder.external_size();
            obj.set_rust_native_state(holder, rt);
            if external_size > 0 {
                obj.set_external_memory_pressure(external_size, rt);
            }
            #[cfg(debug_assertions)]
            {
                // a private marker so debuggers can tell these objects apart
                // from plain empty objects (Nitro's `__type` pattern)
                let mut d = JsiObject::new(rt);
                d.set(PropName::new("value", rt), &JsiValue::new_string(&format!("RustTurboModule<{name}>"), rt), rt);
                d.set(PropName::new("writable", rt), &JsiValue::new_bool(false), rt);
                d.set(PropName::new("enumerable", rt), &JsiValue::new_bool(true), rt);
                d.set(PropName::new("configurable", rt), &JsiValue::new_bool(true), rt);
                define_property(rt, &obj, "__type", &d)?;
            }
            state.instances.insert(name.clone(), rt.clone(&obj));
            Ok(obj.into_value(rt))
        }),
        rt,
    );

    proxy.set(PropName::new("get", rt), &get.as_value(rt), rt);

    // diagnostics (Nitro's HybridNitroModulesProxy pattern)
    let names = JsiFn::from_host_fn(
        &PropName::new("names", rt),
        0,
        Box::new(move |_this, _args, rt| {
            let names: Vec<String> = registry::registered_names().into_iter().map(Into::into).collect();
            Ok(names.into_value(rt))
        }),
        rt,
    );
    proxy.set(PropName::new("names", rt), &names.as_value(rt), rt);

    let has = JsiFn::from_host_fn(
        &PropName::new("has", rt),
        1,
        Box::new(move |_this, mut args, rt| {
            let name: String = match args.len() {
                1 => args
                    .remove(0)
                    .try_into_js(rt)
                    .context("RustTurbo.has expects a module name string")?,
                _ => bail!("RustTurbo.has expects exactly one argument (the module name)"),
            };
            Ok(JsiValue::new_bool(registry::lookup(&name).is_some()))
        }),
        rt,
    );
    proxy.set(PropName::new("has", rt), &has.as_value(rt), rt);

    proxy.set(PropName::new("version", rt), &JsiValue::new_string(env!("CARGO_PKG_VERSION"), rt), rt);
    proxy.set(
        PropName::new("buildType", rt),
        &JsiValue::new_string(if cfg!(debug_assertions) { "debug" } else { "release" }, rt),
        rt,
    );

    rt.global().set(PropName::new("RustTurbo", rt), &proxy.as_value(rt), rt);
    Ok(())
}

#[derive(Default)]
struct PropBuilder {
    method: Option<JsiFn<'static>>,
    get: Option<JsiFn<'static>>,
    set: Option<JsiFn<'static>>,
}

/// Creates the prototype object for a module type: one non-writable data
/// property per method, one accessor descriptor per getter/setter pair.
/// Properties are installed with `Object.defineProperty` (RN 0.88 jsi.h has
/// no C++ API for it — jsi.cpp itself calls the JS function).
fn build_prototype(
    table: &'static MethodTable,
    dispatcher: &Dispatcher,
    rt: &mut RuntimeHandle<'static>,
) -> anyhow::Result<JsiObject<'static>> {
    let proto = JsiObject::new(rt);

    let mut props: BTreeMap<&'static str, PropBuilder> = BTreeMap::new();
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

/// Creates the host function installed on the prototype for one method
/// table entry. The closure resolves `this` through the object's Rust
/// native state on every call.
fn make_entry_fn(
    table: &'static MethodTable,
    entry: &'static MethodEntry,
    dispatcher: Dispatcher,
    rt: &mut RuntimeHandle<'static>,
) -> JsiFn<'static> {
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

fn define_property(
    rt: &mut RuntimeHandle<'static>,
    target: &JsiObject<'static>,
    name: &str,
    descriptor: &JsiObject<'static>,
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
