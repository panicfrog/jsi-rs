//! P3: JS callback parameters — `TurboCallback` captures a JS function so
//! Rust can fire it later (event subscription pattern).

mod common;

use common::create_raw_runtime;
use jsi::{JsiValue, RuntimeHandle};
use jsi_turbo::{install, Dispatcher, RustTurboModule, TurboCallback};

fn with_rt(f: impl FnOnce(&mut RuntimeHandle<'static>, &mut cxx::UniquePtr<jsi_sys::Runtime>)) {
    let mut raw = create_raw_runtime();
    let ptr = unsafe { raw.as_mut().unwrap().get_unchecked_mut() as *mut _ };
    let mut rt = jsi::init(ptr);
    f(&mut rt, &mut raw);
}

fn eval<'rt>(
    _rt: &mut RuntimeHandle<'rt>,
    raw: &mut cxx::UniquePtr<jsi_sys::Runtime>,
    js: &str,
) -> JsiValue<'rt> {
    JsiValue::from_raw(jsi_tests::ffi::bridge::eval_js(raw.pin_mut(), js))
}

/// A module that keeps callbacks and fires them on demand.
pub struct EventBus {
    listeners: std::sync::Mutex<Vec<TurboCallback>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self { listeners: std::sync::Mutex::new(Vec::new()) }
    }
}

#[RustTurboModule("EventBus")]
impl EventBus {
    /// Registers a JS function; the Rust side can call it from any thread.
    pub fn on(&self, cb: TurboCallback) -> anyhow::Result<()> {
        self.listeners.lock().unwrap().push(cb);
        Ok(())
    }

    /// Fires every listener with `value`, returns the listener count.
    pub fn emit(&self, value: f64) -> anyhow::Result<usize> {
        let listeners = self.listeners.lock().unwrap();
        for cb in listeners.iter() {
            cb.call(value);
        }
        Ok(listeners.len())
    }

    /// Fires every listener with a struct (serde-serialized argument).
    pub fn emit_named(&self, name: String, value: f64) -> anyhow::Result<usize> {
        let listeners = self.listeners.lock().unwrap();
        for cb in listeners.iter() {
            cb.call(NamedEvent { name: name.clone(), value });
        }
        Ok(listeners.len())
    }

    /// Fires with no arguments.
    pub fn ping(&self) -> anyhow::Result<usize> {
        let listeners = self.listeners.lock().unwrap();
        for cb in listeners.iter() {
            cb.call_with_no_args();
        }
        Ok(listeners.len())
    }
}

#[derive(serde::Serialize)]
pub struct NamedEvent {
    pub name: String,
    pub value: f64,
}

#[test]
fn turbo_callback_events() {
    with_rt(|rt, raw| {
        install(rt, Dispatcher::Inline).unwrap();

        // no listeners yet
        assert_eq!(0., eval(rt, raw, "RustTurbo.get('EventBus').emit(1)").into_js::<f64>(rt));

        // subscribe and collect events in JS
        eval(
            rt,
            raw,
            r#"globalThis.events = [];
               RustTurbo.get('EventBus').on(v => events.push('n:' + v));"#,
        );
        assert_eq!(1., eval(rt, raw, "RustTurbo.get('EventBus').emit(42)").into_js::<f64>(rt));
        assert_eq!(
            "n:42",
            eval(rt, raw, "events.join(',')").into_js::<String>(rt)
        );

        // struct argument arrives as an object
        eval(
            rt,
            raw,
            r#"globalThis.named = null;
               RustTurbo.get('EventBus').on(e => { named = e.name + '=' + e.value; });"#,
        );
        eval(rt, raw, "RustTurbo.get('EventBus').emitNamed('tick', 7)").into_js::<f64>(rt);
        assert_eq!(
            "tick=7",
            eval(rt, raw, "globalThis.named").into_js::<String>(rt)
        );

        // no-argument invocation
        eval(
            rt,
            raw,
            r#"globalThis.pings = 0;
               RustTurbo.get('EventBus').on(() => { pings++; });"#,
        );
        eval(rt, raw, "RustTurbo.get('EventBus').ping()").into_js::<f64>(rt);
        assert_eq!(1., eval(rt, raw, "globalThis.pings").into_js::<f64>(rt));
    });
}

#[test]
fn turbo_callback_errors_clearly() {
    with_rt(|rt, raw| {
        install(rt, Dispatcher::Inline).unwrap();

        // non-function argument
        let out = eval(
            rt,
            raw,
            r#"(() => { try { RustTurbo.get('EventBus').on(42); return 'no error'; } catch (e) { return e.message; } })()"#,
        );
        let msg: String = out.into_js(rt);
        assert!(msg.contains("`cb`") && msg.contains("not a function"), "got: {msg}");

        // a throwing JS callback does not propagate to the emitter
        eval(rt, raw, r#"RustTurbo.get('EventBus').on(() => { throw new Error('boom'); });"#);
        assert_eq!(
            1.,
            eval(rt, raw, "RustTurbo.get('EventBus').ping()").into_js::<f64>(rt)
        );
    });
}

#[test]
fn turbo_callback_ts_signature() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("generated")
        .join("rust-turbo")
        .join("EventBus.ts");
    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("TS file missing: {e}"));
    assert!(content.contains("on(cb: (arg: any) => void): void;"), "{content}");
    assert!(content.contains("emit(value: number): number;"), "{content}");
}
