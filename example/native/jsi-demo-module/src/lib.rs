//! A demo native module for React Native, written in Rust with jsi-rs.
//!
//! Two generations coexist here on purpose:
//! - `RustDemo`: a legacy `#[host_object]` global installed by
//!   `jsi_demo_install` (the original jsi-rs pattern, kept as a regression
//!   example)
//! - `TurboDemo`: a `#[RustTurboModule]` module, auto-registered into the
//!   linkme slice and lazily served by `globalThis.RustTurbo.get(...)`,
//!   with TypeScript bindings generated at compile time under
//!   `generated/rust-turbo/`

use jsi::{host_object, RuntimeHandle};
use jsi_turbo::{RustTurboModule, TurboStruct};
use serde::{Deserialize, Serialize};

/// Installs the `RustDemo` host object into the JS global scope of the given
/// `jsi::Runtime`.
///
/// Called by the platform glue whenever React Native creates a runtime: the
/// iOS TurboModule's `installJSIBindingsWithRuntime:` and the Android
/// TurboModule's `getBindingsInstaller()` installer. The call invoker is not
/// needed for this demo (all calls are synchronous), so we pass a null one.
#[no_mangle]
pub extern "C" fn jsi_demo_install(runtime: *mut jsi::sys::Runtime) -> bool {
    let (mut rt, _invoker) = jsi::init(runtime, cxx::SharedPtr::null());

    let demo = RustDemo {
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    rt.global().set(
        jsi::PropName::new("RustDemo", &mut rt),
        &jsi::IntoValue::into_value(demo, &mut rt),
        &mut rt,
    );

    true
}

/// A struct that crosses the JSI boundary in both directions via serde.
/// `TurboStruct` additionally generates FromValue/IntoValue impls and a TS
/// interface (`generated/rust-turbo/structs/Point.ts`).
#[derive(Serialize, Deserialize, TurboStruct, PartialEq, Debug)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub label: String,
}

struct RustDemo {
    version: String,
}

#[host_object]
impl RustDemo {
    /// A plain method with two arguments.
    pub fn add(&self, _rt: &mut RuntimeHandle, a: f64, b: f64) -> anyhow::Result<f64> {
        Ok(a + b)
    }

    /// A method that takes and returns a string.
    pub fn greet(&self, _rt: &mut RuntimeHandle, name: String) -> anyhow::Result<String> {
        Ok(format!("Hello, {name}, from Rust!"))
    }

    /// A method that receives a JS array of numbers and aggregates it.
    pub fn sum_list(&self, _rt: &mut RuntimeHandle, nums: Vec<f64>) -> anyhow::Result<f64> {
        Ok(nums.iter().sum())
    }

    /// A method returning a serde struct, which JS sees as a plain object.
    pub fn make_point<'rt>(
        &self,
        rt: &mut RuntimeHandle<'rt>,
        x: f64,
        y: f64,
        label: String,
    ) -> anyhow::Result<jsi::JsiValue<'rt>> {
        let point = Point { x, y, label };
        use jsi::SerializeValue;
        point.serialize_value(rt).map_err(Into::into)
    }

    /// A method receiving a JS object back as the same serde struct.
    pub fn point_magnitude<'rt>(
        &self,
        rt: &mut RuntimeHandle<'rt>,
        value: jsi::JsiValue<'rt>,
    ) -> anyhow::Result<f64> {
        use jsi::DeserializeValue;
        let point: Point = Point::deserialize_value(value, rt)?;
        Ok((point.x * point.x + point.y * point.y).sqrt())
    }

    /// A getter: JS reads `RustDemo.version` like a property.
    #[host_object(getter)]
    pub fn get_version(&self, _rt: &mut RuntimeHandle) -> anyhow::Result<String> {
        Ok(self.version.clone())
    }
}

// ---------------------------------------------------------------------------
// The TurboModule generation: one attribute is the whole spec — the method
// table, linkme registration, force anchor and TypeScript bindings are all
// generated from this impl block.

#[derive(Default)]
pub struct TurboDemo;

#[RustTurboModule("TurboDemo")]
impl TurboDemo {
    pub fn add(&self, _rt: &mut RuntimeHandle, a: f64, b: f64) -> anyhow::Result<f64> {
        Ok(a + b)
    }

    pub fn greet(&self, name: String) -> anyhow::Result<String> {
        Ok(format!("Hello, {name}, from Rust TurboModule!"))
    }

    pub fn sum_list(&self, nums: Vec<f64>) -> anyhow::Result<f64> {
        Ok(nums.iter().sum())
    }

    /// Returns a typed struct; JS receives a plain object and the generated
    /// TS bindings type it as `Point`.
    pub fn make_point(&self, label: String, x: f64, y: f64) -> anyhow::Result<Point> {
        Ok(Point { x, y, label })
    }

    pub fn point_magnitude(&self, p: Point) -> anyhow::Result<f64> {
        Ok((p.x * p.x + p.y * p.y).sqrt())
    }

    /// Async methods are exposed as JS Promises; the body runs on the
    /// jsi-turbo worker pool and resolution hops back to the JS thread.
    pub async fn slow_add(&self, a: f64, b: f64) -> anyhow::Result<f64> {
        Ok(a + b)
    }

    #[turbo(getter)]
    pub fn get_version(&self) -> anyhow::Result<String> {
        Ok(env!("CARGO_PKG_VERSION").into())
    }
}
