//! The per-runtime `RustTurbo` proxy installed as a JS global.
//!
//! `globalThis.RustTurbo.get(name)` lazily constructs a registered module and
//! returns a plain JS object whose prototype carries pre installed methods
//! (Nitro-style): the object itself holds the Rust [`ModuleHolder`] as
//! `jsi::NativeState`, and every method call resolves `this` through that
//! state. All per-runtime caches live in the `get` host-function's closure,
//! so they die with the runtime.

use std::collections::HashMap;

use anyhow::{bail, Context};
use jsi::{AsValue, IntoValue, JsiFn, JsiObject, JsiValue, PropName, RuntimeHandle};

use crate::dispatcher::Dispatcher;
use crate::prototype;
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
                    let p = prototype::build(table, &dispatcher, rt)?;
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
                prototype::define_property(rt, &obj, "__type", &d)?;
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
