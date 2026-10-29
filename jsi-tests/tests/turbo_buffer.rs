//! P3: ArrayBuffer zero-copy — Rust -> JS via external buffers (ownership
//! handoff), JS -> Rust via borrowed `JsiArrayBuffer` parameters.

mod common;

use common::create_raw_runtime;
use jsi::{JsiArrayBuffer, JsiValue, RuntimeHandle};
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
pub struct BufferUtils;

#[RustTurboModule("BufferUtils")]
impl BufferUtils {
    /// Moves a Rust allocation into JS as an ArrayBuffer backed by that
    /// same memory (no copy). Pattern: byte i = i mod 251.
    pub fn make_buffer<'rt>(&self, rt: &mut RuntimeHandle<'rt>, len: f64) -> anyhow::Result<JsiArrayBuffer<'rt>> {
        let data: Vec<u8> = (0..len as usize).map(|i| (i % 251) as u8).collect();
        Ok(JsiArrayBuffer::from_vec(data, rt))
    }

    /// Sum of the bytes of a JS-provided buffer, read in place.
    pub fn checksum<'rt>(&self, rt: &mut RuntimeHandle<'rt>, buf: JsiArrayBuffer<'rt>) -> anyhow::Result<f64> {
        Ok(buf.data(rt).iter().map(|&b| b as u64).sum::<u64>() as f64)
    }

    /// Mutates the caller's buffer in place — JS sees the writes, proving
    /// the memory is shared rather than copied.
    pub fn fill<'rt>(&self, rt: &mut RuntimeHandle<'rt>, buf: JsiArrayBuffer<'rt>, value: f64) -> anyhow::Result<()> {
        let value = value as u8;
        for b in buf.data(rt) {
            *b = value;
        }
        Ok(())
    }
}

#[test]
fn turbo_buffer_rust_to_js_zero_copy() {
    with_rt(|rt, raw| {
        install(rt, Dispatcher::Inline).unwrap();

        // byteLength and contents arrive intact
        assert_eq!(
            256.,
            eval(rt, raw, "RustTurbo.get('BufferUtils').makeBuffer(256).byteLength").into_js::<f64>(rt)
        );
        // pattern check: byte i = i mod 251
        assert_eq!(
            "0,1,2,3,4",
            eval(
                rt,
                raw,
                "Array.from(new Uint8Array(RustTurbo.get('BufferUtils').makeBuffer(1024), 0, 5)).join(',')"
            )
            .into_js::<String>(rt)
        );
        // an external buffer is a real ArrayBuffer
        assert_eq!(
            "true",
            eval(
                rt,
                raw,
                "String(RustTurbo.get('BufferUtils').makeBuffer(8) instanceof ArrayBuffer)"
            )
            .into_js::<String>(rt)
        );
    });
}

#[test]
fn turbo_buffer_js_to_rust_borrowed() {
    with_rt(|rt, raw| {
        install(rt, Dispatcher::Inline).unwrap();

        // JS-built buffer read in place by Rust (sum of 0..16 = 120)
        assert_eq!(
            120.,
            eval(
                rt,
                raw,
                r#"(() => {
                     const b = new ArrayBuffer(16);
                     const u8 = new Uint8Array(b);
                     for (let i = 0; i < 16; i++) u8[i] = i;
                     return RustTurbo.get('BufferUtils').checksum(b);
                   })()"#
            )
            .into_js::<f64>(rt)
        );

        // Rust writes into the caller's buffer are visible in JS
        assert_eq!(
            "7,7,7,7",
            eval(
                rt,
                raw,
                r#"(() => {
                     const b = new ArrayBuffer(4);
                     RustTurbo.get('BufferUtils').fill(b, 7);
                     return Array.from(new Uint8Array(b)).join(',');
                   })()"#
            )
            .into_js::<String>(rt)
        );

        // non-ArrayBuffer arguments produce a conversion error
        let out = eval(
            rt,
            raw,
            r#"(() => { try { RustTurbo.get('BufferUtils').checksum('not a buffer'); return 'no error'; } catch (e) { return e.message; } })()"#,
        );
        let msg: String = out.into_js(rt);
        assert!(msg.contains("`buf`"), "got: {msg}");
    });
}

#[test]
fn turbo_buffer_roundtrip_no_copy() {
    with_rt(|rt, raw| {
        install(rt, Dispatcher::Inline).unwrap();

        // Rust -> JS -> Rust without copying: make 1 KiB, checksum it back
        // (sum of i mod 251 for i in 0..1024)
        let expected: u64 = (0..1024usize).map(|i| (i % 251) as u64).sum();
        assert_eq!(
            expected as f64,
            eval(
                rt,
                raw,
                "RustTurbo.get('BufferUtils').checksum(RustTurbo.get('BufferUtils').makeBuffer(1024))"
            )
            .into_js::<f64>(rt)
        );

        // the same at 4 KiB with a hard-coded expected value (what the
        // example app displays)
        assert_eq!(
            505160.,
            eval(
                rt,
                raw,
                "RustTurbo.get('BufferUtils').checksum(RustTurbo.get('BufferUtils').makeBuffer(4096))"
            )
            .into_js::<f64>(rt)
        );
    });
}

#[test]
fn turbo_buffer_ts_signature() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("generated")
        .join("rust-turbo")
        .join("BufferUtils.ts");
    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("TS file missing: {e}"));
    assert!(content.contains("makeBuffer(len: number): ArrayBuffer;"), "{content}");
    assert!(content.contains("checksum(buf: ArrayBuffer): number;"), "{content}");
}
