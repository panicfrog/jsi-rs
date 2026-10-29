//! Phase 2: async module methods exposed as JS Promises, driven on the real
//! Hermes harness with the Inline dispatcher (single-threaded settle).

mod common;

use common::create_raw_runtime;
use jsi::{JsiValue, RuntimeHandle};
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
) -> JsiValue<'rt> {
    JsiValue::from_raw(jsi_tests::ffi::bridge::eval_js(raw.pin_mut(), js))
}

#[derive(Default)]
pub struct AsyncOps;

#[RustTurboModule("AsyncOps")]
impl AsyncOps {
    pub async fn slow_add(&self, a: f64, b: f64) -> anyhow::Result<f64> {
        Ok(a + b)
    }

    pub async fn fetch_items(&self, prefix: String) -> anyhow::Result<Vec<String>> {
        Ok(vec![format!("{prefix}-1"), format!("{prefix}-2")])
    }

    pub async fn maybe_inc(&self, v: Option<f64>) -> anyhow::Result<Option<f64>> {
        Ok(v.map(|x| x + 1.))
    }

    pub async fn finish(&self) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn fail(&self) -> anyhow::Result<f64> {
        anyhow::bail!("deliberate turbo failure")
    }
}

#[test]
fn turbo_async_resolves() {
    with_rt(|rt, raw| {
        install(rt, Dispatcher::Inline).unwrap();

        rt.global().set(
            jsi::PropName::new("result", rt),
            &jsi::JsiValue::new_null(),
            rt,
        );
        eval(
            rt,
            raw,
            "RustTurbo.get('AsyncOps').slowAdd(19, 23).then(v => { result = v; });",
        );

        // Inline dispatcher settles synchronously; the .then callback is a
        // microtask
        raw.pin_mut().drain_microtasks(-1);
        assert_eq!(42., eval(rt, raw, "result").into_js::<f64>(rt));

        // Vec + String return
        eval(
            rt,
            raw,
            "RustTurbo.get('AsyncOps').fetchItems('item').then(v => { result = v.join(','); });",
        );
        raw.pin_mut().drain_microtasks(-1);
        assert_eq!("item-1,item-2", eval(rt, raw, "result").into_js::<String>(rt));

        // Option round-trip
        eval(
            rt,
            raw,
            "RustTurbo.get('AsyncOps').maybeInc(null).then(v => { result = String(v); });",
        );
        raw.pin_mut().drain_microtasks(-1);
        assert_eq!("null", eval(rt, raw, "result").into_js::<String>(rt));
        eval(
            rt,
            raw,
            "RustTurbo.get('AsyncOps').maybeInc(41).then(v => { result = v; });",
        );
        raw.pin_mut().drain_microtasks(-1);
        assert_eq!(42., eval(rt, raw, "result").into_js::<f64>(rt));

        // unit return resolves to undefined
        eval(
            rt,
            raw,
            "RustTurbo.get('AsyncOps').finish().then(() => { result = 'done'; });",
        );
        raw.pin_mut().drain_microtasks(-1);
        assert_eq!("done", eval(rt, raw, "result").into_js::<String>(rt));
    });
}

#[test]
fn turbo_async_rejects() {
    with_rt(|rt, raw| {
        install(rt, Dispatcher::Inline).unwrap();

        rt.global().set(
            jsi::PropName::new("err", rt),
            &jsi::JsiValue::new_null(),
            rt,
        );
        eval(
            rt,
            raw,
            "RustTurbo.get('AsyncOps').fail().catch(e => { err = e && e.message; });",
        );
        raw.pin_mut().drain_microtasks(-1);

        let out = eval(
            rt,
            raw,
            "typeof err === 'string' ? err : String(err)",
        );
        let msg: String = out.into_js(rt);
        assert!(
            msg.contains("deliberate turbo failure"),
            "rejection should carry the error message, got: {msg:?}"
        );
    });
}

#[test]
fn turbo_async_ts_signatures() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("generated")
        .join("rust-turbo")
        .join("AsyncOps.ts");
    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("TS file missing: {e}"));
    assert!(content.contains("slowAdd(a: number, b: number): Promise<number>;"), "{content}");
    assert!(content.contains("fetchItems(prefix: string): Promise<string[]>;"), "{content}");
    assert!(content.contains("maybeInc(v: number | null): Promise<number | null>;"), "{content}");
    assert!(content.contains("finish(): Promise<void>;"), "{content}");
}
