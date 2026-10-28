//! Phase 1 vertical slice: `#[RustTurboModule]` macro → linkme registry →
//! lazy `RustTurbo.get(name)` → prototype method dispatch → TS file emitted.

mod common;

use common::create_raw_runtime;
use jsi::{JsiValue, RuntimeHandle};
use jsi_turbo::{
    install, Dispatcher, MethodEntry, MethodKind, MethodTable, ModuleDescriptor, ModuleHolder,
    RustTurboModule, TurboModuleObject,
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

fn js_error_message(rt: &mut RuntimeHandle<'_>, raw: &mut cxx::UniquePtr<jsi_sys::Runtime>, js: &str) -> String {
    let out = eval(
        rt,
        raw,
        &format!(r#"(() => {{ try {{ {js} }} catch (e) {{ return e.message }} }})()"#),
    );
    out.into_js(rt)
}

#[derive(Default)]
pub struct MathUtils;

#[RustTurboModule("MathUtils")]
impl MathUtils {
    pub fn add(&self, _rt: &mut RuntimeHandle, a: f64, b: f64) -> anyhow::Result<f64> {
        Ok(a + b)
    }

    // no runtime handle needed for pure computations
    pub fn echo(&self, s: String) -> anyhow::Result<String> {
        Ok(s)
    }

    pub fn sum(&self, _rt: &mut RuntimeHandle, nums: Vec<f64>) -> anyhow::Result<f64> {
        Ok(nums.iter().sum())
    }

    pub fn maybe_double(&self, _rt: &mut RuntimeHandle, v: Option<f64>) -> anyhow::Result<Option<f64>> {
        Ok(v.map(|x| x * 2.))
    }

    pub fn poke(&self) -> anyhow::Result<()> {
        Ok(())
    }

    #[turbo(getter)]
    pub fn get_version(&self, _rt: &mut RuntimeHandle) -> anyhow::Result<String> {
        Ok("1.0".into())
    }
}

#[test]
fn turbo_vertical_slice() {
    with_rt(|rt, raw| {
        install(rt, Dispatcher::Inline).unwrap();

        // the proxy is installed as a global
        assert_eq!(
            "object",
            eval(rt, raw, "typeof RustTurbo").into_js::<String>(rt)
        );

        // method call through the prototype
        assert_eq!(42., eval(rt, raw, "RustTurbo.get('MathUtils').add(19, 23)").into_js::<f64>(rt));

        // lazy singleton: same instance per name
        assert!(
            eval(rt, raw, "RustTurbo.get('MathUtils') === RustTurbo.get('MathUtils')").into_js::<bool>(rt)
        );

        // getter exposed as a property
        assert_eq!("1.0", eval(rt, raw, "RustTurbo.get('MathUtils').version").into_js::<String>(rt));

        // methods without a runtime handle, Vec and Option params
        assert_eq!(
            "round trip",
            eval(rt, raw, "RustTurbo.get('MathUtils').echo('round trip')").into_js::<String>(rt)
        );
        assert_eq!(6., eval(rt, raw, "RustTurbo.get('MathUtils').sum([1, 2, 3])").into_js::<f64>(rt));
        assert_eq!(10., eval(rt, raw, "RustTurbo.get('MathUtils').maybeDouble(5)").into_js::<f64>(rt));
        assert_eq!(
            "null",
            eval(rt, raw, "String(RustTurbo.get('MathUtils').maybeDouble(null))").into_js::<String>(rt)
        );

        // unit return
        assert_eq!(
            "undefined",
            eval(rt, raw, "String(RustTurbo.get('MathUtils').poke())").into_js::<String>(rt)
        );

        // methods cannot be reassigned (non-writable, non-configurable)
        assert_eq!(
            "false",
            eval(
                rt,
                raw,
                "(() => { 'use strict'; try { RustTurbo.get('MathUtils').add = 1; return 'true'; } catch (e) { return 'false'; } })()"
            )
            .into_js::<String>(rt)
        );

        // unknown module error lists registered names
        let msg = js_error_message(rt, raw, "RustTurbo.get('Nope')");
        assert!(msg.contains("Nope") && msg.contains("MathUtils"), "got: {msg}");

        // argument conversion errors carry module.method and argument names
        let msg = js_error_message(rt, raw, r#"RustTurbo.get('MathUtils').add("not a number", 1)"#);
        assert!(msg.contains("MathUtils.add"), "got: {msg}");
        assert!(msg.contains("`a`"), "got: {msg}");

        // calling a method on a destructured `this` produces a friendly error
        let msg = js_error_message(
            rt,
            raw,
            r#"(() => { const { add } = RustTurbo.get('MathUtils'); return add(1, 2); })()"#,
        );
        assert!(
            msg.contains("not an object") || msg.contains("Rust state") || msg.contains("destructure"),
            "got: {msg}"
        );
    });
}

// A module registered manually (no macro), proving the manual path works
// alongside auto-registration.
pub struct ManualModule {
    value: f64,
}

impl Default for ManualModule {
    fn default() -> Self {
        Self { value: 7. }
    }
}

fn manual_value<'rt>(
    holder: &ModuleHolder,
    _dispatcher: &jsi_turbo::Dispatcher,
    rt: &mut RuntimeHandle<'rt>,
    _args: &[JsiValue<'rt>],
) -> anyhow::Result<JsiValue<'rt>> {
    use jsi::IntoValue;
    let this: std::sync::Arc<ManualModule> = holder
        .downcast()
        .ok_or_else(|| anyhow::anyhow!("wrong module type"))?;
    Ok(this.value.into_value(rt))
}

static MANUAL_TABLE: MethodTable = MethodTable {
    module_name: "Manual",
    rust_type: "ManualModule",
    entries: &[MethodEntry {
        js_name: "value",
        kind: MethodKind::Method,
        param_count: 0,
        invoke: manual_value,
    }],
};

impl TurboModuleObject for ManualModule {
    fn __turbo_construct() -> anyhow::Result<Self> {
        Ok(Default::default())
    }
    fn __turbo_method_table() -> &'static MethodTable {
        &MANUAL_TABLE
    }
}

#[test]
fn turbo_manual_registration() {
    jsi_turbo::register(ModuleDescriptor {
        name: "Manual",
        ctor: || ModuleHolder::new::<ManualModule>(),
    });

    with_rt(|rt, raw| {
        install(rt, Dispatcher::Inline).unwrap();
        assert_eq!(7., eval(rt, raw, "RustTurbo.get('Manual').value()").into_js::<f64>(rt));
    });
}

#[test]
fn turbo_ts_file_emitted() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("generated")
        .join("rust-turbo")
        .join("MathUtils.ts");
    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("TS file missing: {e}"));
    assert!(content.contains("export interface MathUtils {"), "{content}");
    assert!(content.contains("add(a: number, b: number): number;"), "{content}");
    assert!(content.contains("readonly version: string;"), "{content}");
    assert!(content.contains("export function getMathUtils(): MathUtils"), "{content}");
    assert!(content.contains("sum(nums: number[]): number;"), "{content}");
    assert!(content.contains("maybeDouble(v: number | null): number | null;"), "{content}");
}
