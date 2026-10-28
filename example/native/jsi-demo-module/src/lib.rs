//! A demo native module for React Native, written in Rust with jsi-rs.
//!
//! This is a realistic example of what a user-facing jsi-rs library looks
//! like: it installs a `RustDemo` host object into the JS global scope,
//! exposing methods, getters and serde-backed struct conversion.

use jsi::{host_object, IntoValue, PropName, RuntimeHandle};
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
        PropName::new("RustDemo", &mut rt),
        &demo.into_value(&mut rt),
        &mut rt,
    );

    true
}

/// A struct that crosses the JSI boundary in both directions via serde.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
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
