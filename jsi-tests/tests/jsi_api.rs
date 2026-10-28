mod common;

use common::create_raw_runtime;
use jsi::{
    host_object, FromObject, IntoValue, JsiFn, JsiObject, JsiString, JsiValue, PropName,
    RuntimeHandle, UserHostObject,
};

fn with_rt(f: impl FnOnce(&mut RuntimeHandle<'static>, &mut cxx::UniquePtr<jsi_sys::Runtime>)) {
    let mut raw = create_raw_runtime();
    let ptr = unsafe { raw.as_mut().unwrap().get_unchecked_mut() as *mut _ };
    let (mut rt, _invoker) = jsi::init(ptr, cxx::SharedPtr::null());
    f(&mut rt, &mut raw);
}

fn eval<'rt>(
    _rt: &mut RuntimeHandle<'rt>,
    raw: &mut cxx::UniquePtr<jsi_sys::Runtime>,
    js: &str,
) -> JsiValue<'rt> {
    JsiValue::from_raw(jsi_tests::ffi::bridge::eval_js(raw.pin_mut(), js))
}

#[test]
fn value_constructors() {
    with_rt(|rt, _| {
        assert!(JsiValue::new_undefined().is_undefined());
        assert!(JsiValue::new_null().is_null());
        assert!(!JsiValue::new_null().is_undefined());
        assert!(JsiValue::new_bool(true).is_bool());
        assert!(JsiValue::new_number(1.5).is_number());
        assert!(JsiValue::new_string("hi", rt).is_string());
    });
}

#[test]
fn global_roundtrip() {
    with_rt(|rt, raw| {
        rt.global()
            .set(PropName::new("meaning", rt), &JsiValue::new_number(42.), rt);
        let out = eval(rt, raw, "meaning");
        let n: f64 = out.into_js(rt);
        assert_eq!(42., n);

        rt.global()
            .set(PropName::new("greeting", rt), &JsiValue::new_string("hello", rt), rt);
        let out = eval(rt, raw, "greeting");
        let s: String = out.into_js(rt);
        assert_eq!("hello", s);
    });
}

#[test]
fn vec_to_array() {
    with_rt(|rt, raw| {
        let nums: Vec<f64> = vec![1., 2., 3.];
        rt.global().set(PropName::new("nums", rt), &nums.into_value(rt), rt);

        let out = eval(rt, raw, "nums.length");
        assert_eq!(3., out.into_js::<f64>(rt));

        let out = eval(rt, raw, "nums[0] + nums[2]");
        assert_eq!(4., out.into_js::<f64>(rt));

        let back: Vec<f64> = eval(rt, raw, "nums").into_js(rt);
        assert_eq!(vec![1., 2., 3.], back);
    });
}

struct ExampleHostObject {
    secret: f64,
}

#[host_object]
impl ExampleHostObject {
    pub fn time(&self, _rt: &mut RuntimeHandle) -> anyhow::Result<f64> {
        Ok(3200.)
    }

    #[host_object(getter)]
    pub fn get_secret(&self, _rt: &mut RuntimeHandle) -> anyhow::Result<f64> {
        Ok(self.secret)
    }

    pub fn add(&self, _rt: &mut RuntimeHandle, a: f64, b: f64) -> anyhow::Result<f64> {
        Ok(a + b)
    }

    pub fn echo(&self, _rt: &mut RuntimeHandle, s: String) -> anyhow::Result<String> {
        Ok(s)
    }
}

#[test]
fn host_object_macro() {
    with_rt(|rt, raw| {
        let ho = ExampleHostObject { secret: 99. };
        rt.global().set(PropName::new("Example", rt), &ho.into_value(rt), rt);

        // method (snake_case -> camelCase doesn't apply to `time`/`add`/`echo`)
        let out = eval(rt, raw, "Example.time()");
        assert_eq!(3200., out.into_js::<f64>(rt));

        let out = eval(rt, raw, "Example.add(19, 23)");
        assert_eq!(42., out.into_js::<f64>(rt));

        let out = eval(rt, raw, "Example.echo('round trip')");
        assert_eq!("round trip", out.into_js::<String>(rt));

        // getter
        let out = eval(rt, raw, "Example.secret");
        assert_eq!(99., out.into_js::<f64>(rt));
    });
}

#[test]
fn call_js_function_from_rust() {
    with_rt(|rt, raw| {
        // define a JS function, then call it from Rust
        eval(rt, raw, "function triple(x) { return x * 3; }");

        let triple = rt.global().get(PropName::new("triple", rt), rt);
        let triple: JsiObject = triple.try_into_js(rt).unwrap();
        let triple = JsiFn::from_object(&triple, rt).unwrap();

        let arg = JsiValue::new_number(14.);
        let out = triple.call([arg], rt).unwrap();
        assert_eq!(42., out.into_js::<f64>(rt));
    });
}

#[test]
fn user_host_object_trait() {
    struct Manual;

    impl<'rt> UserHostObject<'rt> for Manual {
        fn get(
            &mut self,
            name: jsi::PropName<'rt>,
            rt: &mut RuntimeHandle<'rt>,
        ) -> anyhow::Result<JsiValue<'rt>> {
            match rt.to_string(&name).as_str() {
                "answer" => Ok(JsiValue::new_number(42.)),
                _ => Ok(JsiValue::new_undefined()),
            }
        }

        fn set(
            &mut self,
            _name: jsi::PropName<'rt>,
            _value: JsiValue<'rt>,
            _rt: &mut RuntimeHandle<'rt>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        fn properties(&mut self, rt: &mut RuntimeHandle<'rt>) -> Vec<jsi::PropName<'rt>> {
            vec![PropName::new("answer", rt)]
        }
    }

    with_rt(|rt, raw| {
        rt.global().set(PropName::new("Manual", rt), &Manual.into_value(rt), rt);
        let out = eval(rt, raw, "Manual.answer");
        assert_eq!(42., out.into_js::<f64>(rt));

        let out = eval(rt, raw, "Object.keys(Manual).join(',')");
        assert_eq!("answer", out.into_js::<String>(rt));
    });
}

#[test]
fn serde_roundtrip() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Point {
        x: f64,
        y: f64,
        label: String,
        tags: Vec<String>,
    }

    with_rt(|rt, _| {
        let p = Point {
            x: 1.5,
            y: -2.5,
            label: "origin".into(),
            tags: vec!["a".into(), "b".into()],
        };

        use jsi::SerializeValue;
        let val = p.serialize_value(rt).unwrap();
        rt.global().set(PropName::new("point", rt), &val, rt);

        use jsi::DeserializeValue;
        let got: Point = Point::deserialize_value(val, rt).unwrap();
        assert_eq!(p, got);
    });
}

#[test]
fn string_display() {
    with_rt(|rt, _| {
        let s = JsiString::new("héllo wörld", rt);
        assert_eq!("héllo wörld", rt.to_string(&s));
    });
}

// Regression: the macro used to generate unqualified `PropName` / `jsi::...`
// paths, forcing users to import them. This module deliberately imports
// nothing but the macro itself.
mod macro_hygiene {
    use jsi::host_object;

    pub struct MinimalHostObject;

    #[host_object]
    impl MinimalHostObject {
        pub fn compute(&self, _rt: &mut jsi::RuntimeHandle, x: f64) -> anyhow::Result<f64> {
            Ok(x * 2.)
        }
    }
}

#[test]
fn host_object_macro_hygiene() {
    with_rt(|rt, raw| {
        let ho = macro_hygiene::MinimalHostObject;
        rt.global()
            .set(PropName::new("Minimal", rt), &ho.into_value(rt), rt);

        let out = eval(rt, raw, "Minimal.compute(21)");
        assert_eq!(42., out.into_js::<f64>(rt));
    });
}

#[test]
fn host_object_bad_args_error_message() {
    with_rt(|rt, raw| {
        let ho = ExampleHostObject { secret: 1. };
        rt.global()
            .set(PropName::new("BadArgs", rt), &ho.into_value(rt), rt);

        // calling with a string where a number is expected must throw a JS
        // error whose message names the offending argument (regression: it
        // used to print the literal "$ arg_names")
        let out = eval(
            rt,
            raw,
            r#"(() => { try { BadArgs.add("not a number", 1); return "NO ERROR" } catch (e) { return e.message } })()"#,
        );
        let msg: String = out.into_js(rt);
        assert!(msg.contains("a"), "error message should name argument 'a', got: {msg}");
        assert!(!msg.contains("$"), "error message should not contain '$', got: {msg}");
    });
}

mod events {
    use jsi::HostEvent;

    #[derive(Debug, PartialEq)]
    pub enum TestEvent {
        Started,
        Progress { percent: f64 },
        Finished { ok: bool, label: String },
    }

    impl<'rt> HostEvent<'rt> for TestEvent {
        type Key = TestEventKey;

        fn key(&self) -> TestEventKey {
            match self {
                TestEvent::Started => TestEventKey::Started,
                TestEvent::Progress { .. } => TestEventKey::Progress,
                TestEvent::Finished { .. } => TestEventKey::Finished,
            }
        }

        fn args(self, rt: &mut jsi::RuntimeHandle<'rt>) -> Vec<jsi::JsiValue<'rt>> {
            use jsi::IntoValue;
            match self {
                TestEvent::Started => vec![],
                TestEvent::Progress { percent } => vec![percent.into_value(rt)],
                TestEvent::Finished { ok, label } => vec![ok.into_value(rt), label.into_value(rt)],
            }
        }
    }

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
    pub enum TestEventKey {
        Started,
        Progress,
        Finished,
    }

    impl std::str::FromStr for TestEventKey {
        type Err = anyhow::Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "started" => Ok(TestEventKey::Started),
                "progress" => Ok(TestEventKey::Progress),
                "finished" => Ok(TestEventKey::Finished),
                _ => anyhow::bail!("invalid event name"),
            }
        }
    }

    impl jsi::HostEventKey for TestEventKey {}
}

#[test]
fn host_event_trait() {
    with_rt(|rt, _| {
        use events::*;
        use jsi::HostEvent;

        // key() maps variants to the key enum
        assert_eq!(TestEventKey::Started, TestEvent::Started.key());
        assert_eq!(TestEventKey::Progress, TestEvent::Progress { percent: 50. }.key());

        // args() converts payloads to JS values
        let args = TestEvent::Finished { ok: true, label: "done".into() }.args(rt);
        assert_eq!(2, args.len());
        assert_eq!(true, args[0].into_js::<bool>(rt));
        assert_eq!("done", args[1].into_js::<String>(rt));

        // the key enum parses from its camelCase name (what the derive
        // generates for `#[derive(HostEvent)]`)
        assert_eq!(TestEventKey::Finished, "finished".parse().unwrap());
        assert!("nope".parse::<TestEventKey>().is_err());
    });
}

// exercises the actual `#[derive(HostEvent)]` macro end-to-end
mod derived_events {
    use jsi::HostEvent;

    #[derive(HostEvent, Debug)]
    pub enum DerivedEvent {
        Connected,
        Message { text: String },
        Disconnected { reason: String, code: f64 },
    }
}

#[test]
fn host_event_derive() {
    with_rt(|rt, _| {
        use derived_events::*;
        use jsi::HostEvent;

        // the derive generates the companion Key enum with camelCase parsing
        assert_eq!(DerivedEventKey::Connected, "connected".parse().unwrap());
        assert_eq!(DerivedEventKey::Message, "message".parse().unwrap());
        assert!("bogus".parse::<DerivedEventKey>().is_err());

        assert_eq!(DerivedEventKey::Connected, DerivedEvent::Connected.key());

        // args() converts each payload field into a JsiValue
        let args = DerivedEvent::Disconnected { reason: "bye".into(), code: 42. }.args(rt);
        assert_eq!(2, args.len());
        assert_eq!("bye", args[0].into_js::<String>(rt));
        assert_eq!(42., args[1].into_js::<f64>(rt));

        let args = DerivedEvent::Message { text: "hi".into() }.args(rt);
        assert_eq!(1, args.len());
        assert_eq!("hi", args[0].into_js::<String>(rt));
    });
}
