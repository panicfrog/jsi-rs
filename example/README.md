# jsi-rs example app

A complete React Native app that integrates a **Rust JSI module** through jsi-rs.
The Rust module lives in [`native/jsi-demo-module`](native/jsi-demo-module) and
exposes a `RustDemo` host object to JavaScript:

```ts
RustDemo.version        // getter
RustDemo.add(19, 23)    // method with arguments
RustDemo.greet("jsi-rs")            // string round trip
RustDemo.sumList([1, 2, 3])         // JS array -> Rust Vec
RustDemo.makePoint(3, 4, "label")   // serde struct -> JS object
RustDemo.pointMagnitude(point)      // JS object -> serde struct
```

This example always matches the React Native version vendored by this branch
of jsi-rs (see the crate versions in the workspace root).

## Prerequisites

- Rust (`rustup`), with the targets you need:
  - iOS: `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`
  - Android: `rustup target add aarch64-linux-android` and [cargo-ndk]
- Xcode + CocoaPods (iOS), Android SDK + NDK + JDK 21 (Android)

## Build the Rust library

The app links **one** Rust aggregate library (`native/rust-app`) that contains
every Rust module plus the [jsi-turbo](../jsi-turbo) runtime. Building it
also runs `#[RustTurboModule]` macros, which write the TypeScript bindings
under `native/jsi-demo-module/generated/rust-turbo/`.

```bash
# from the repo root

# iOS simulator
cargo build --release -p rust-app --target aarch64-apple-ios-sim
cp target/aarch64-apple-ios-sim/release/librust_app.a \
   example/native/jsi-demo-module/librust-app.a

# Android (arm64 emulator / device) — also a static library, linked into
# libappmodules.so by the app's CMake
cargo ndk build -t arm64-v8a -P 24 --release -p rust-app
mkdir -p example/android/app/src/main/rust/arm64-v8a
cp target/aarch64-linux-android/release/librust_app.a \
   example/android/app/src/main/rust/arm64-v8a/
```

For iOS device builds, also build `aarch64-apple-ios` and either lipo the two
`.a` files together or update the podspec.

## Run on iOS

```bash
cd ios
pod install
# open JsiDemo.xcworkspace in Xcode, or:
xcodebuild -workspace JsiDemo.xcworkspace -scheme JsiDemo \
  -destination 'id=<SIMULATOR_UDID>' -derivedDataPath build
xcrun simctl install <SIMULATOR_UDID> build/Build/Products/Debug-iphonesimulator/JsiDemo.app
xcrun simctl launch <SIMULATOR_UDID> org.reactjs.native.example.JsiDemo
```

Tip: use `xcrun simctl list devices` to find the simulator UDID; prefer the
UDID over the name, since names can match multiple simulator runtimes.

## Run on Android

```bash
cd android
JAVA_HOME=<path-to-jdk-21> ./gradlew assembleDebug
adb install app/build/outputs/apk/debug/app-debug.apk
adb reverse tcp:8081 tcp:8081   # let the app reach metro
adb shell am start -n com.jsidemo/.MainActivity
```

Tip: start the emulator with `-gpu host` — the default software renderer on
some hosts shows a black window while the guest is actually fine.

## Start metro

```bash
npm start
```

## How the integration works

- **`TurboDemo` — `#[RustTurboModule]` (the jsi-turbo framework)**: the impl
  block in [`native/jsi-demo-module/src/lib.rs`](native/jsi-demo-module/src/lib.rs)
  is the entire spec — the macro generates the method table, linkme
  auto-registration and the TypeScript bindings. The platform glue
  ([iOS `RustTurboModule.mm`](native/jsi-demo-module/ios/RustTurboModule.mm),
  [Android `RustTurboBindings.cpp`](android/app/src/main/jni/RustTurboBindings.cpp))
  is a one-time-per-app carrier that hands every new JS runtime to
  `rust_turbo_install` in the aggregate library; JS gets modules via the
  generated `getTurboDemo()` accessor, which lazily constructs the module
  through the `RustTurbo` proxy (`globalThis.RustTurbo.get('TurboDemo')`).
  React Native invokes the carrier when the `RustTurbo` TurboModule is first
  requested — the generated TS bindings do that automatically.

[cargo-ndk]: https://github.com/bbqsrc/cargo-ndk
