# react-native-rust-turbo

Rust TurboModules for React Native — the core package (platform carriers +
build hooks). Write your module as a plain Rust impl block; a single
attribute macro is the entire spec:

```rust
#[RustTurboModule("MathUtils")]
impl MathUtils {
    pub fn add(&self, _rt: &mut RuntimeHandle, a: f64, b: f64) -> anyhow::Result<f64> {
        Ok(a + b)
    }
    pub async fn fetch(&self, url: String) -> anyhow::Result<String> { /* ... */ }
    #[turbo(getter)]
    pub fn get_version(&self) -> anyhow::Result<String> { /* ... */ }
}
```

```ts
// generated at compile time — the only JS-side entry point
import { getMathUtils } from 'my-math-utils/generated/rust-turbo/MathUtils';
```

The macro generates the native method table, auto-registers the module,
and writes the TypeScript bindings. No bridge, no per-module platform code.

## Requirements

- React Native **0.88.x** (new architecture). jsi.h is the ABI contract
  with the engine, so this package must match your RN line — patch
  releases (0.88.x) are fine, other minors may not be.
- Rust via [rustup](https://rustup.rs) with the targets for your platforms
  (`rustup target add aarch64-apple-ios-sim aarch64-apple-ios
  aarch64-linux-android`)
- Xcode + CocoaPods (iOS), Android SDK/NDK + JDK 21 (Android)

No jsi-rs checkout needed — the framework's Rust sources ship inside this
package (`rust/`), and jsi.h comes from your app's own react-native.

## Installation

```bash
npm install react-native-rust-turbo <your-module-packages>
cd ios && pod install
```

That's it — no files to copy, no Podfile/CMakeLists edits. The build hooks
below run cargo automatically; editing Rust code just works on the next
normal build.

## How it works

- **iOS**: autolinking discovers the `RustTurbo` pod. Its script phase
  runs before compile, discovers every installed rust-turbo module
  package, generates one aggregate cargo crate, and builds it into a
  single static library linked into the app. Platforms and configurations
  map to Rust targets automatically.
- **Android**: autolinking registers the library module. The
  `rustTurboBuild` gradle task runs cargo before CMake (NDK toolchain
  injected — cargo-ndk not needed); ABIs follow
  `reactNativeArchitectures` in `gradle.properties`.
- **Version guard**: the build warns if the app's react-native and the
  jsi-rs checkout drift apart in major.minor.

## Writing module packages

Scaffold one:

```bash
npx react-native-rust-turbo init my-math-utils
```

A module package is a normal npm package: one Rust crate + npm metadata
declaring its modules:

```json
{
  "name": "my-math-utils",
  "rust-turbo": {
    "crate": ".",
    "crateName": "my-math-utils",
    "modules": ["MathUtils"]
  }
}
```

Its `Cargo.toml` depends on the framework crates from this package:
`jsi = { path = "../react-native-rust-turbo/rust/jsi" }` — one copy in
the cargo graph.

Full guide — method/type rules, TurboStruct, zero-copy ArrayBuffer,
nested objects, JS callbacks, publishing:
[docs/usage.md](https://github.com/panicfrog/jsi-rs/blob/main/jsi-turbo/docs/usage.md)
(中文版:
[usage.zh-CN.md](https://github.com/panicfrog/jsi-rs/blob/main/jsi-turbo/docs/usage.zh-CN.md)).
A complete runnable example lives in the repo's `example/`.

## Troubleshooting

| Symptom | Cause |
|---|---|
| `RustTurbo bindings failed to install` / RustTurbo pod missing | the npm dependency graph is invalid (a wrong `file:` path) — RN CLI silently drops such packages; check with `npm ls` |
| `RustTurbo.get('X')` says not registered | the package's `modules` list is missing that module (iOS FORCE_LINK anchor — the linker stripped it) |
| `no matching package named ... found` (cargo) | `crateName` doesn't match the crate's real name in Cargo.toml |
| Android undefined symbol | the aggregate library must be a staticlib, not a cdylib |
| `could not find jsi/jsi.h` | cargo was run outside the platform hooks; jsi.h comes from the app's react-native via the generated `.cargo/config.toml` |

## License

MIT — see [jsi-rs](https://github.com/panicfrog/jsi-rs). Originally forked
from [laptou/jsi-rs](https://github.com/laptou/jsi-rs).
