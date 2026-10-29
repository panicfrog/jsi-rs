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

use jsi::{JsiArrayBuffer, RuntimeHandle};
use jsi_turbo::{RustTurboModule, TurboCallback, TurboStruct};
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

    /// Moves a Rust allocation into JS as an ArrayBuffer backed by that
    /// same memory — zero-copy handoff (the GC drops the Rust Vec when the
    /// buffer becomes unreachable). Pattern: byte i = i mod 251.
    pub fn make_buffer<'rt>(
        &self,
        rt: &mut RuntimeHandle<'rt>,
        len: f64,
    ) -> anyhow::Result<JsiArrayBuffer<'rt>> {
        let data: Vec<u8> = (0..len as usize).map(|i| (i % 251) as u8).collect();
        Ok(JsiArrayBuffer::from_vec(data, rt))
    }

    /// Reads a JS-provided buffer in place (no copy) and sums its bytes.
    pub fn checksum<'rt>(
        &self,
        rt: &mut RuntimeHandle<'rt>,
        buf: JsiArrayBuffer<'rt>,
    ) -> anyhow::Result<f64> {
        Ok(buf.data(rt).iter().map(|&b| b as u64).sum::<u64>() as f64)
    }
}

/// An event-source module: JS subscribes with a function, Rust calls it
/// later from wherever it likes (the call hops back to the JS thread via
/// the CallInvoker).
#[derive(Default)]
pub struct Ticker {
    listeners: std::sync::Mutex<Vec<TurboCallback>>,
}

#[RustTurboModule("Ticker")]
impl Ticker {
    pub fn on_tick(&self, cb: TurboCallback) -> anyhow::Result<()> {
        self.listeners.lock().unwrap().push(cb);
        Ok(())
    }

    /// Fires `count` ticks at each listener immediately (on the JS thread).
    /// Returns the listener count.
    pub fn fire_ticks(&self, count: f64) -> anyhow::Result<usize> {
        let n = self.listeners.lock().unwrap().len();
        self.fire(count, std::time::Duration::ZERO);
        Ok(n)
    }

    /// Same, but the firing loop runs on the worker pool with a one-second
    /// gap between ticks — the UI can be seen updating as each event hops
    /// back to the JS thread through the dispatcher.
    pub async fn fire_ticks_async(&self, count: f64) -> anyhow::Result<usize> {
        let n = self.listeners.lock().unwrap().len();
        self.fire(count, std::time::Duration::from_secs(1));
        Ok(n)
    }
}

impl Ticker {
    fn fire(&self, count: f64, interval: std::time::Duration) {
        for i in 0..count as usize {
            if i > 0 {
                std::thread::sleep(interval);
            }
            let listeners = self.listeners.lock().unwrap();
            for cb in listeners.iter() {
                cb.call(TickEvent {
                    index: i as f64,
                    total: count,
                    at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as f64,
                });
            }
        }
    }
}

#[derive(Serialize)]
pub struct TickEvent {
    pub index: f64,
    pub total: f64,
    pub at: f64,
}
