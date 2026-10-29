//! A demo native module for React Native, written in Rust with jsi-turbo.
//!
//! `#[RustTurboModule("TurboDemo")]` on the impl block below is the entire
//! spec: the method table, linkme auto-registration, iOS force anchor and
//! TypeScript bindings (`generated/rust-turbo/`) are all generated from it.
//! JS accesses the module through the generated accessor:
//!
//! ```ts
//! import { getTurboDemo } from '<...>/generated/rust-turbo/TurboDemo';
//! getTurboDemo().add(19, 23); // 42
//! ```

use jsi::RuntimeHandle;
use jsi_turbo::{RustTurboModule, TurboStruct};
use serde::{Deserialize, Serialize};

/// A struct that crosses the JSI boundary in both directions via serde.
/// `TurboStruct` additionally generates FromValue/IntoValue impls and a TS
/// interface (`generated/rust-turbo/structs/Point.ts`).
#[derive(Serialize, Deserialize, TurboStruct, PartialEq, Debug)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub label: String,
}

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
