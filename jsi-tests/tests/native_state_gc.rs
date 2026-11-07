//! NativeState GC reclamation — the ground truth this framework's
//! lifetime story stands on: dropping the last JS reference and running
//! a collection must run the Rust destructor (on Hermes, via the
//! NativeState cell's finalizer on the background finalizer thread).
//!
//! This covers the whole GC-visible chain for every consumer of the
//! pattern (TurboObject's ModuleHolder included: cell finalize →
//! shared_ptr<NativeState> → Box<RustNativeCarrier> → the payload; the
//! holder-to-Arc step below that is plain Rust refcounting, no GC
//! involved). If this test fails, every "the GC drops the native object"
//! claim built on jsi-rs is void — and nothing below the JS side can fix
//! it there.

mod common;

use common::create_raw_runtime;
use jsi::{AsValue, JsiObject, PropName, RuntimeHandle};
use jsi_turbo::{install, Dispatcher, RustTurboModule};

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
) -> jsi::JsiValue<'rt> {
    jsi::JsiValue::from_raw(jsi_tests::ffi::bridge::eval_js(raw.pin_mut(), js))
}

static DROPPED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

struct Payload;

impl Drop for Payload {
    fn drop(&mut self) {
        DROPPED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// A module whose method returns a plain `JsiObject` carrying a
/// NativeState — the exact shape monet-rn's app-side diagnostic used,
/// and a return path the direct test above does not cover (the invoker
/// converts the returned object, the test publishes it by hand).
#[derive(Default)]
pub struct NativeThingFactory;

#[RustTurboModule("NativeThingFactory")]
impl NativeThingFactory {
    pub fn make<'rt>(
        &self,
        rt: &mut RuntimeHandle<'rt>,
    ) -> anyhow::Result<JsiObject<'rt>> {
        let obj = JsiObject::new(rt);
        obj.set_rust_native_state(Payload, rt);
        Ok(obj)
    }
}

/// The cell's finalizer queues onto Hermes' finalizer thread — poll for
/// it rather than assuming synchronous reclamation.
fn wait_for_drops(expected: usize, timeout_ms: u64) -> bool {
    let mut waited = 0u64;
    while DROPPED.load(std::sync::atomic::Ordering::Relaxed) < expected
        && waited < timeout_ms
    {
        std::thread::sleep(std::time::Duration::from_millis(10));
        waited += 10;
    }
    DROPPED.load(std::sync::atomic::Ordering::Relaxed) >= expected
}

#[test]
fn native_state_payload_is_reclaimed_by_gc() {
    with_rt(|rt, raw| {
        assert_eq!(
            eval(rt, raw, "typeof gc").into_js::<String>(rt),
            "function",
            "the host Hermes must expose gc() for this test to mean anything"
        );

        let obj = JsiObject::new(rt);
        obj.set_rust_native_state(Payload, rt);
        rt.global()
            .set(PropName::new("holder", rt), &obj.as_value(rt), rt);
        drop(obj);

        // Drop the only JS-side reference: not collected yet (no
        // collection has run since).
        eval(rt, raw, "holder = undefined");
        assert_eq!(
            DROPPED.load(std::sync::atomic::Ordering::Relaxed),
            0,
            "the payload dropped before any collection could have seen it"
        );

        // Collect, with some garbage in the same pass so the collector
        // has real work (Nitro's memory tests use 55k objects for the
        // same reason; a single lonely cell could otherwise ride several
        // young collections).
        eval(
            rt,
            raw,
            "let junk = []; for (let i = 0; i < 10_000; i++) junk.push({x: i}); junk = undefined; gc();",
        );
        assert!(
            wait_for_drops(1, 5_000),
            "gc() did not reclaim the NativeState's Rust payload — the \
             finalizer chain (cell finalize → shared_ptr → Box) never ran"
        );
    });
}

/// The same claim through a module method's RETURN value: the invoker
/// hands the wrapped object to JS (the path app code actually takes),
/// JS drops it, a collection must run the payload's Drop.
#[test]
fn native_state_object_returned_from_method_is_reclaimed_by_gc() {
    with_rt(|rt, raw| {
        install(rt, Dispatcher::Inline).unwrap();

        let baseline = DROPPED.load(std::sync::atomic::Ordering::Relaxed);
        eval(
            rt,
            raw,
            "globalThis.thing = RustTurbo.get('NativeThingFactory').make(); 'ok'",
        );
        eval(rt, raw, "globalThis.thing = undefined; gc();");

        let mut waited = 0u64;
        while DROPPED.load(std::sync::atomic::Ordering::Relaxed) <= baseline
            && waited < 5_000
        {
            std::thread::sleep(std::time::Duration::from_millis(10));
            waited += 10;
        }
        assert!(
            DROPPED.load(std::sync::atomic::Ordering::Relaxed) > baseline,
            "gc() did not reclaim a NativeState object returned from a \
             module method — the invoker's return path retains it"
        );
    });
}
