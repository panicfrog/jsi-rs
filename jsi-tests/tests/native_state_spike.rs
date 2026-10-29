//! Phase 0 spike: jsi::NativeState round-trip through the new jsi-sys
//! bindings. This validates the cxx bridge shape (RustNativeState holding a
//! Rust object behind Box<dyn Any>) that the jsi-turbo framework builds on.

mod common;

use common::create_raw_runtime;
use jsi::{AsValue, JsiObject, JsiValue, PropName, RuntimeHandle};

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

#[derive(Debug, PartialEq)]
struct Payload(i32, String);

#[test]
fn native_state_roundtrip() {
    with_rt(|rt, _| {
        let obj = JsiObject::new(rt);
        assert!(!obj.has_native_state(rt));

        obj.set_rust_native_state(Payload(7, "seven".into()), rt);
        assert!(obj.has_native_state(rt));

        let state = obj.get_rust_native_state(rt).expect("should be rust state");
        let payload = state.get::<Payload>().expect("should downcast");
        assert_eq!(&Payload(7, "seven".into()), payload);
    });
}

#[test]
fn plain_objects_have_no_state() {
    with_rt(|rt, raw| {
        let val = eval(rt, raw, "({ answer: 42 })");
        let obj: JsiObject = val.try_into_js(rt).unwrap();
        assert!(!obj.has_native_state(rt));
        assert!(obj.get_rust_native_state(rt).is_none());
    });
}

#[test]
fn clearing_state_disposes() {
    with_rt(|rt, _| {
        let obj = JsiObject::new(rt);
        obj.set_rust_native_state(Payload(1, "one".into()), rt);
        assert!(obj.get_rust_native_state(rt).is_some());

        // dispose, Nitro-style: overwrite with null state. NOTE: Hermes keeps
        // reporting hasNativeState == true for the (now empty) slot; what
        // matters is that the Rust object is no longer reachable.
        obj.clear_native_state(rt);
        assert!(obj.get_rust_native_state(rt).is_none());
    });
}

// The Nitro pattern skeleton: a plain object created with a prototype that
// has pre-installed methods, carrying Rust state. When JS calls a prototype
// method, `this` is the plain object and its native state resolves to the
// Rust object.
#[test]
fn prototype_object_with_rust_state() {
    with_rt(|rt, raw| {
        // prototype with a method that reads `this.name`
        let proto = eval(rt, raw, "({ greet: function() { return 'hi ' + this.name; } })");

        let mut obj = JsiObject::create_with_prototype(&proto, rt);
        obj.set(PropName::new("name", rt), &JsiValue::new_string("rust", rt), rt);
        obj.set_rust_native_state(Payload(11, "eleven".into()), rt);

        // the state survives while the object is passed through JS
        rt.global()
            .set(PropName::new("holder", rt), &obj.as_value(rt), rt);
        let out = eval(rt, raw, "holder.greet()");
        assert_eq!("hi rust", out.into_js::<String>(rt));

        // prototype method call did not disturb the native state, and `this`
        // (the object JS sees) still resolves to our Rust payload
        assert!(obj.has_native_state(rt));
        let state = obj.get_rust_native_state(rt).unwrap();
        assert_eq!(&Payload(11, "eleven".into()), state.get::<Payload>().unwrap());

        // external memory pressure is accepted silently
        obj.set_external_memory_pressure(1024, rt);
    });
}
