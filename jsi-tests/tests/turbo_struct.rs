//! Phase 2: #[derive(TurboStruct)] — serde-backed typed structs crossing the
//! TurboModule boundary, including from async methods.

mod common;

use common::create_raw_runtime;
use jsi::{JsiValue, RuntimeHandle};
use jsi_turbo::{install, Dispatcher, RustTurboModule, TurboStruct};
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, TurboStruct, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub label: String,
}

#[derive(Default)]
pub struct Geometry;

#[RustTurboModule("Geometry")]
impl Geometry {
    pub fn make_point(&self, label: String, x: f64, y: f64) -> anyhow::Result<Point> {
        Ok(Point { x, y, label })
    }

    pub fn magnitude(&self, p: Point) -> anyhow::Result<f64> {
        Ok((p.x * p.x + p.y * p.y).sqrt())
    }

    pub fn total(&self, ps: Vec<Point>) -> anyhow::Result<f64> {
        Ok(ps.iter().map(|p| p.x + p.y).sum())
    }

    pub async fn translate(&self, p: Point, dx: f64, dy: f64) -> anyhow::Result<Point> {
        Ok(Point {
            x: p.x + dx,
            y: p.y + dy,
            label: p.label,
        })
    }
}

#[test]
fn turbo_struct_roundtrip() {
    with_rt(|rt, raw| {
        install(rt, Dispatcher::Inline).unwrap();

        // Rust struct returned to JS as a plain object
        let out = eval(rt, raw, "RustTurbo.get('Geometry').makePoint('origin', 3, 4)");
        let obj = out.try_into_js::<jsi::JsiObject>(rt).unwrap();
        assert_eq!(
            "origin",
            obj.get(jsi::PropName::new("label", rt), rt).into_js::<String>(rt)
        );
        let x = obj.get(jsi::PropName::new("x", rt), rt);
        assert_eq!(3., x.into_js::<f64>(rt));

        // JS object literal into Rust struct param
        assert_eq!(
            5.,
            eval(rt, raw, "RustTurbo.get('Geometry').magnitude({ x: 3, y: 4, label: 'a' })")
                .into_js::<f64>(rt)
        );

        // Vec<Point>
        assert_eq!(
            30.,
            eval(
                rt,
                raw,
                "RustTurbo.get('Geometry').total([{x: 1, y: 2, label: 'a'}, {x: 10, y: 17, label: 'b'}])"
            )
            .into_js::<f64>(rt)
        );

        // async method taking and returning a struct
        rt.global()
            .set(jsi::PropName::new("result", rt), &jsi::JsiValue::new_null(), rt);
        eval(
            rt,
            raw,
            "RustTurbo.get('Geometry').translate({x: 1, y: 1, label: 'p'}, 10, 20).then(p => { result = p.label + ':' + p.x + ',' + p.y; });",
        );
        raw.pin_mut().drain_microtasks(-1);
        assert_eq!("p:11,21", eval(rt, raw, "result").into_js::<String>(rt));
    });
}

#[test]
fn turbo_struct_ts_files() {
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("generated")
        .join("rust-turbo");

    let point = std::fs::read_to_string(base.join("structs").join("Point.ts"))
        .unwrap_or_else(|e| panic!("Point.ts missing: {e}"));
    assert!(point.contains("export interface Point {"), "{point}");
    assert!(point.contains("x: number;"), "{point}");
    assert!(point.contains("label: string;"), "{point}");

    let geo = std::fs::read_to_string(base.join("Geometry.ts"))
        .unwrap_or_else(|e| panic!("Geometry.ts missing: {e}"));
    assert!(
        geo.contains("import type { Point } from './structs/Point';"),
        "{geo}"
    );
    assert!(geo.contains("makePoint(label: string, x: number, y: number): Point;"), "{geo}");
    assert!(geo.contains("magnitude(p: Point): number;"), "{geo}");
    assert!(geo.contains("total(ps: Point[]): number;"), "{geo}");
    assert!(geo.contains("translate(p: Point, dx: number, dy: number): Promise<Point>;"), "{geo}");
}
