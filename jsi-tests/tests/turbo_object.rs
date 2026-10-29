//! P3: nested objects (object graphs) — `#[TurboObject]` values returned
//! from module methods, held by JS, and passed back by reference.
//! Design: `jsi-turbo/docs/nested-objects.md`.

mod common;

use common::create_raw_runtime;
use jsi::{JsiValue, RuntimeHandle};
use jsi_turbo::{install, Dispatcher, RustTurboModule, TurboObject};

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

fn js_error_message(rt: &mut RuntimeHandle<'_>, raw: &mut cxx::UniquePtr<jsi_sys::Runtime>, js: &str) -> String {
    let out = eval(
        rt,
        raw,
        &format!(r#"(() => {{ try {{ {js} }} catch (e) {{ return e.message }} }})()"#),
    );
    out.into_js(rt)
}

/// A nested object: created by Canvas, held by JS, passed back by
/// reference. Interior mutability like any module (methods take &self).
pub struct Layer {
    name: String,
    points: std::sync::Mutex<Vec<(f64, f64)>>,
    opacity: std::sync::atomic::AtomicU8,
}

impl Default for Layer {
    fn default() -> Self {
        Self {
            name: String::new(),
            points: std::sync::Mutex::new(Vec::new()),
            opacity: std::sync::atomic::AtomicU8::new(100),
        }
    }
}

#[TurboObject]
impl Layer {
    pub fn add_point(&self, x: f64, y: f64) -> anyhow::Result<()> {
        self.points.lock().unwrap().push((x, y));
        Ok(())
    }

    pub fn point_count(&self) -> anyhow::Result<f64> {
        Ok(self.points.lock().unwrap().len() as f64)
    }

    #[turbo(getter)]
    pub fn get_name(&self) -> anyhow::Result<String> {
        Ok(self.name.clone())
    }

    #[turbo(getter)]
    pub fn get_opacity(&self) -> anyhow::Result<f64> {
        Ok(self.opacity.load(std::sync::atomic::Ordering::Relaxed) as f64 / 100.)
    }

    #[turbo(setter)]
    pub fn set_opacity(&self, value: f64) -> anyhow::Result<()> {
        self.opacity.store((value * 100.) as u8, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

#[derive(Default)]
pub struct Canvas {
    created: std::sync::atomic::AtomicUsize,
}

#[RustTurboModule("Canvas")]
impl Canvas {
    pub fn create_layer(&self, _rt: &mut RuntimeHandle, name: String) -> anyhow::Result<Layer> {
        self.created.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(Layer { name, ..Default::default() })
    }

    /// Takes the object back — the same Rust instance, by reference.
    pub fn count_points(&self, _rt: &mut RuntimeHandle, layer: std::sync::Arc<Layer>) -> anyhow::Result<f64> {
        layer.point_count()
    }

    #[turbo(getter)]
    pub fn get_layers_created(&self) -> anyhow::Result<f64> {
        Ok(self.created.load(std::sync::atomic::Ordering::Relaxed) as f64)
    }
}

#[test]
fn turbo_object_roundtrip() {
    with_rt(|rt, raw| {
        install(rt, Dispatcher::Inline).unwrap();

        // module returns a nested object; JS calls methods on it
        assert_eq!(
            "bg",
            eval(rt, raw, "RustTurbo.get('Canvas').createLayer('bg').name").into_js::<String>(rt)
        );
        assert_eq!(
            2.,
            eval(
                rt,
                raw,
                r#"(() => {
                     const l = RustTurbo.get('Canvas').createLayer('fg');
                     l.addPoint(1, 2);
                     l.addPoint(3, 4);
                     return l.pointCount();
                   })()"#
            )
            .into_js::<f64>(rt)
        );

        // passing back by reference: the same instance the JS object wraps
        assert_eq!(
            2.,
            eval(
                rt,
                raw,
                r#"(() => {
                     globalThis.held = RustTurbo.get('Canvas').createLayer('held');
                     held.addPoint(5, 6);
                     held.addPoint(7, 8);
                     return RustTurbo.get('Canvas').countPoints(held);
                   })()"#
            )
            .into_js::<f64>(rt)
        );
        // state survives across calls (the object is alive in JS)
        assert_eq!(
            2.,
            eval(rt, raw, "RustTurbo.get('Canvas').countPoints(globalThis.held)").into_js::<f64>(rt)
        );

        // getter/setter pair on the nested object
        assert_eq!(1., eval(rt, raw, "globalThis.held.opacity").into_js::<f64>(rt));
        eval(rt, raw, "globalThis.held.opacity = 0.5");
        assert_eq!(0.5, eval(rt, raw, "globalThis.held.opacity").into_js::<f64>(rt));

        // the module kept count of constructions
        assert_eq!(3., eval(rt, raw, "RustTurbo.get('Canvas').layersCreated").into_js::<f64>(rt));
    });
}

#[test]
fn turbo_object_errors() {
    with_rt(|rt, raw| {
        install(rt, Dispatcher::Inline).unwrap();

        // destructured `this` produces the friendly shared error
        let msg = js_error_message(
            rt,
            raw,
            r#"(() => { const { addPoint } = RustTurbo.get('Canvas').createLayer('x'); return addPoint(1, 2); })()"#,
        );
        assert!(
            msg.contains("not an object") || msg.contains("Rust state") || msg.contains("destructure"),
            "got: {msg}"
        );

        // passing something that is not a wrapped object fails conversion
        let msg = js_error_message(
            rt,
            raw,
            r#"RustTurbo.get('Canvas').countPoints({ points: 4 })"#,
        );
        assert!(msg.contains("`layer`"), "got: {msg}");

        // debug builds tag instances with __type
        if cfg!(debug_assertions) {
            assert_eq!(
                "RustTurboObject<Layer>",
                eval(rt, raw, "RustTurbo.get('Canvas').createLayer('t').__type").into_js::<String>(rt)
            );
        }
    });
}

#[test]
fn turbo_object_ts_files() {
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("generated")
        .join("rust-turbo");

    let layer = std::fs::read_to_string(base.join("structs").join("Layer.ts"))
        .unwrap_or_else(|e| panic!("Layer.ts missing: {e}"));
    assert!(layer.contains("// @rust-turbo generated by #[TurboObject]"), "{layer}");
    assert!(layer.contains("export interface Layer {"), "{layer}");
    assert!(layer.contains("addPoint(x: number, y: number): void;"), "{layer}");
    assert!(layer.contains("readonly name: string;"), "{layer}");
    assert!(layer.contains("opacity: number;"), "{layer}");

    // the module interface imports the object type
    let canvas = std::fs::read_to_string(base.join("Canvas.ts"))
        .unwrap_or_else(|e| panic!("Canvas.ts missing: {e}"));
    assert!(canvas.contains("import type { Layer } from './structs/Layer';"), "{canvas}");
    assert!(canvas.contains("createLayer(name: string): Layer;"), "{canvas}");
    assert!(canvas.contains("countPoints(layer: Layer): number;"), "{canvas}");
}
