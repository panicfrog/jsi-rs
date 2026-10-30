# jsi-turbo usage guide (local path integration)

Write React Native TurboModules in Rust: a single attribute macro is the
entire spec. The framework generates the native method table, auto-registers
the module, writes TypeScript bindings, and wires the cargo build into the
platform builds — **no manual cargo or copy steps**. This guide covers
local-path (`file:`) integration in the order "wire up an app" → "write a
module" → "troubleshoot"; a complete runnable example is `example/` in this
repository. (中文版:[usage.zh-CN.md](usage.zh-CN.md))

```rust
#[RustTurboModule("MathUtils")]
impl MathUtils {
    pub fn add(&self, _rt: &mut RuntimeHandle, a: f64, b: f64) -> anyhow::Result<f64> { Ok(a + b) }
    pub async fn fetch(&self, url: String) -> anyhow::Result<String> { /* ... */ }
    #[turbo(getter)]
    pub fn get_version(&self) -> anyhow::Result<String> { /* ... */ }
}
```

```ts
// generated at compile time — the only JS-side entry point
import { getMathUtils } from 'my-math-utils/generated/rust-turbo/MathUtils';
```

## 0. Prerequisites

- Rust (rustup; `rustup target add aarch64-apple-ios-sim aarch64-apple-ios
  aarch64-linux-android`), Android SDK/NDK + JDK 21, Xcode + CocoaPods
- A checkout of the jsi-rs repository (no `git submodule update` needed,
  see below)

## 1. Wire up your app (once per app)

### 1.1 Install two packages

```bash
# core package: platform carriers + build hooks (required)
npm install <path-to-jsi-rs>/jsi-turbo/npm

# your module package (see section 2; local path during development)
npm install ./native/my-math-utils
```

The core package is installed via `file:` as a symlink; the build hook
locates the jsi-rs checkout automatically (always found when the core
package is in `node_modules`) — **no environment variables needed**.

### 1.2 iOS

```bash
cd ios && pod install     # autolinking discovers the RustTurbo pod
# Build with Xcode or your CI command as usual; before the first compile
# the pod's script phase runs the cargo aggregate build (platform/config
# automatically mapped to the right Rust target):
xcodebuild -workspace MyApp.xcworkspace -scheme MyApp ...
```

### 1.3 Android

```bash
cd android && ./gradlew assembleDebug
# the rustTurboBuild task runs cargo before CMake (build.js injects the
# NDK toolchain itself, no cargo-ndk needed); ABIs follow
# reactNativeArchitectures in gradle.properties (arm64-only by default)
```

Done. From here on, **changing Rust code only requires a normal build** —
the platform hooks recompile automatically.

## 2. Write a module package

A module package = one Rust crate + npm metadata:

```
my-math-utils/
  package.json     # see below
  Cargo.toml       # crate-type = ["lib"]
  src/lib.rs       # #[RustTurboModule] impl block
  generated/       # macro-generated TS (updated at build time; .gitignore it)
```

Key `package.json` fields:

```json
{
  "name": "my-math-utils",
  "version": "0.1.0",
  "main": "generated/rust-turbo/MathUtils.ts",
  "rust-turbo": {
    "crate": ".",
    "crateName": "my-math-utils",
    "modules": ["MathUtils"]
  }
}
```

- `crate`: the crate directory (relative to the package root; usually `"."`)
- `crateName`: **must exactly match `[package].name` in Cargo.toml**
  (cargo path dependencies match by name — hyphens/underscores are not
  interchangeable)
- `modules`: the names of all `#[RustTurboModule("...")]` modules (used for
  the iOS FORCE_LINK anchors; a missing entry silently drops that module
  on iOS)

`Cargo.toml` dependencies:

```toml
[lib]
crate-type = ["lib"]

[dependencies]
jsi = { path = "<path-to-jsi-rs>/jsi", default-features = false, features = ["macros", "serde"] }
jsi-turbo = { path = "<path-to-jsi-rs>/jsi-turbo" }
linkme = "0.3"
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
```

Method rules in short: `&self` + an optional `rt: &mut RuntimeHandle`
second parameter + `anyhow::Result<T>` return. Supported types:
`f64/i32/u32/bool/String/Vec/Option`, `#[derive(TurboStruct)]` structs,
`JsiArrayBuffer` (zero-copy), `#[TurboObject]` (nested objects),
`TurboCallback` (JS callbacks); `async fn` is exposed as a Promise (JSI
types are rejected in async signatures). Full rules: section 6 below,
the doc comments in `jsi-turbo/src/lib.rs`, and
`example/native/jsi-demo-module`.

## 3. Using it from JS

```ts
import { getMathUtils } from 'my-math-utils/generated/rust-turbo/MathUtils';

const math = getMathUtils();        // lazily constructed on first call
math.add(19, 23);                   // 42
await math.fetch('...');            // Promise
math.version;                       // getter
```

Diagnostics: `RustTurbo.names()` / `.has(name)` / `.version` /
`.buildType` (query registration state straight from the JS console).

## 4. Common errors → causes

| Error / symptom | Cause |
|---|---|
| `RustTurbo bindings failed to install` and `get('RustTurbo')` is null | the core package is not in autolinking: usually a wrong `file:` path in package.json (check with `npm ls` — an `invalid` entry means a bad path; RN CLI drops such packages **silently**) |
| RustTurbo missing from Podfile.lock after pod install | same as above; check `npx @react-native-community/cli config` for the dependency |
| `no matching package named ... found` (cargo) | the module package's `crateName` doesn't match the Cargo.toml package name |
| `RustTurbo.get('X')` says not registered | `modules` list is missing that module (no iOS FORCE_LINK anchor — the linker stripped it) |
| White screen / `"App" has not been registered` | metro isn't running or is serving the wrong project (port 8081 serves one app at a time) |
| Android undefined symbol | the superlib was changed to a cdylib — the aggregate library must be a staticlib |
| `could not find jsi/jsi.h` / React Native directory | the aggregate crate generated by the core package carries a `.cargo/config.toml` pointing at the app's node_modules; running cargo manually outside the platform hooks loses that environment |

## 5. How it works (one diagram)

```
npm install (core package + module packages)
pod install / gradle config (autolinking discovers RustTurbo)
platform build → build hooks (pod script phase / rustTurboBuild task)
  → build.js scans node_modules for all rust-turbo module packages
  → generates the aggregate superlib (<app>/node_modules/.cache/rust-turbo/)
  → one cargo build → a single librust_app.a → linked into platform artifacts
JS import of generated TS → bootstrap triggers the carrier → RustTurbo
  proxy installed → get(name) lazily constructs the module
  (plain object + prototype + NativeState)
```

## 6. Method and type rules (complete)

### 6.1 Method signatures

```rust
#[RustTurboModule("MathUtils")]
impl MathUtils {
    // plain method: &self + any number of arguments (may be none)
    // + a Result return
    pub fn add(&self, a: f64, b: f64) -> anyhow::Result<f64> { Ok(a + b) }

    // pass RuntimeHandle as the second parameter when you need JSI
    // handles (optional, as needed)
    pub fn make_buffer<'rt>(
        &self, rt: &mut RuntimeHandle<'rt>, len: f64,
    ) -> anyhow::Result<JsiArrayBuffer<'rt>> { /* ... */ }

    // getters/setters merge into a JS property; the get_/set_ prefix
    // is stripped from the name
    #[turbo(getter)] pub fn get_version(&self) -> anyhow::Result<String> { /* ... */ }
    #[turbo(setter)] pub fn set_cache_size(&self, size: f64) -> anyhow::Result<()> { /* ... */ }

    // async: the body runs on the worker pool, JS gets a Promise
    pub async fn fetch(&self, url: String) -> anyhow::Result<String> { /* ... */ }
}
```

Rules:

- every method returns `anyhow::Result<T>`; errors surface as JS
  exceptions (with the method name)
- `rt: &mut RuntimeHandle` is only passed when JSI handles are needed
  (e.g. ArrayBuffer); such methods need an explicit `<'rt>` lifetime
- arguments and return values of `async fn` must **not** contain JSI
  types (HRTB constraint, rejected at compile time); use `TurboCallback`
  / `TurboObject` for handles that cross threads
- custom constructor: `#[RustTurboModule("X", ctor = "X::create")]`,
  any `fn() -> anyhow::Result<X>` path — `Default` is not required

### 6.2 Argument / return type cheat sheet

| Rust type | TS type | Notes |
|---|---|---|
| `f64` / `i32` / `u32` / `bool` | `number` / `boolean` | JS number semantics |
| `String` | `string` | |
| `Vec<T>` / `Option<T>` | `T[]` / `T \| null` | nested serde |
| `#[derive(TurboStruct)]` struct | `interface` | deep copy, see 7.1 |
| `JsiArrayBuffer<'rt>` | `ArrayBuffer` | zero-copy, sync methods only, see 7.3 |
| a `TurboObject` type | `interface` | reference semantics, see 7.4 |
| `Arc<T: TurboObject>` argument | same | recovers the same instance from the JS object |
| `TurboCallback` argument | `(arg: T) => void` | a JS function, see 7.5 |

## 7. Advanced features

All of these have runnable examples in
`example/native/jsi-demo-module` and assertions in `jsi-tests/tests/`.

### 7.1 TurboStruct (value-semantics structs)

```rust
#[derive(Serialize, Deserialize, TurboStruct, PartialEq, Debug)]
pub struct Point { pub x: f64, pub y: f64, pub label: String }
```

- deep-copied both ways via serde: JS receives a plain object, passing it
  back deserializes it
- generates the `interface` in `generated/rust-turbo/structs/Point.ts`
- fits small, immutable data; for large or identity-carrying objects use
  7.3 / 7.4

### 7.2 async → Promise

- the method body runs on the jsi-turbo worker pool (never blocks the JS
  thread)
- resolve/reject hop back to the JS thread via the CallInvoker
- demo: `await turbo.slowAdd(20, 22)`

### 7.3 Zero-copy ArrayBuffer

```rust
// Rust → JS: the Vec's ownership is transferred; the JS ArrayBuffer is
// backed by that same memory. The Rust memory is only dropped when the
// GC collects the buffer.
pub fn make_buffer<'rt>(&self, rt: &mut RuntimeHandle<'rt>, len: f64)
    -> anyhow::Result<JsiArrayBuffer<'rt>>
{
    Ok(JsiArrayBuffer::from_vec(vec![0u8; len as usize], rt))
}

// JS → Rust: a borrow; reads/writes happen in place for the duration of
// the synchronous call, visible on both sides
pub fn checksum<'rt>(&self, rt: &mut RuntimeHandle<'rt>,
    buf: JsiArrayBuffer<'rt>) -> anyhow::Result<f64>
{
    Ok(buf.data(rt).iter().map(|&b| b as u64).sum::<u64>() as f64)
}
```

### 7.4 TurboObject (nested objects, reference semantics)

A module method returns a non-singleton Rust object; JS holds it, calls
methods on it, and passes it back — always the same instance:

```rust
pub struct Counter { value: AtomicU64 }

#[TurboObject]
impl Counter {
    pub fn increment(&self, by: f64) -> anyhow::Result<f64> { /* ... */ }
    #[turbo(getter)] pub fn get_value(&self) -> anyhow::Result<f64> { /* ... */ }
}

#[RustTurboModule("Canvas")]
impl Canvas {
    pub fn create_layer(&self, _rt: &mut RuntimeHandle, name: String)
        -> anyhow::Result<Layer> { /* ... */ }
    // Arc<T> argument: recovers the same Arc from the JS object's
    // NativeState
    pub fn count_points(&self, _rt: &mut RuntimeHandle, layer: Arc<Layer>)
        -> anyhow::Result<f64> { /* ... */ }
}
```

- lifetime decided by the JS GC (NativeState destructor → Arc count hits
  zero)
- v1 trade-offs (design notes in [`docs/nested-objects.md`](nested-objects.md)):
  - no JS `===` identity cache (each handoff creates a new JS wrapper)
  - the prototype is rebuilt on each handoff (high-churn scenarios not
    optimized yet)
  - object methods are sync-only (async would need dispatcher plumbing,
    not done)

### 7.5 TurboCallback (JS functions as arguments)

```rust
#[derive(Default)]
pub struct Ticker { listeners: Mutex<Vec<TurboCallback>> }

#[RustTurboModule("Ticker")]
impl Ticker {
    pub fn on_tick(&self, cb: TurboCallback) -> anyhow::Result<()> {
        self.listeners.lock().unwrap().push(cb);
        Ok(())
    }
}

// call back from any thread at any time (arguments are serde-serialized;
// the call hops back to the JS thread via the CallInvoker)
cb.call(TickEvent { index: 1.0, total: 3.0 });
cb.call_with_no_args();
```

- `TurboCallback` is `Send + Sync`; store it in the module for as long
  as you like
- verified (RN 0.88.1, iOS + Android): calling `setState` directly inside
  the callback renders immediately (RuntimeScheduler runs a microtask
  checkpoint + updateRendering after every task) — **no `setTimeout`
  wrapper needed**

## 8. Lifetimes and reload

| Object | Scope | After reload |
|---|---|---|
| module registry (linkme) / worker pool | process | survives |
| `RustTurbo` proxy + module instances | per-runtime | destroyed with the old runtime, reinstalled on cold start |
| TurboObject instances | JS GC | gone with the runtime |
| TurboCallback | lives with the module that captured it | same |

- reload is safe by default: all JS-side state starts from scratch
- if your Rust side holds process-wide resources (caches, thread pools),
  manage them yourself

## 9. Publishing a module package

A module package is a plain npm package. After publishing, consumers only
need:

```bash
npm install my-math-utils
cd ios && pod install
```

Pre-publish checklist:

- `files` in `package.json` includes `src/`, `generated/`, `Cargo.toml`
  (the cargo sources must ship with the package — the aggregate build
  runs inside the **consumer's app**, see the mechanism in section 5)
- the `rust-turbo.modules` list matches the actual macro names one by one
  (iOS anchors; a missing entry silently drops the module)
- `crateName` matches the Cargo.toml package name
- version numbers should mirror the RN version (aligned with jsi-rs's
  versioning policy, 0.88.x)
