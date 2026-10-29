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

## Build the Rust module

The crate is part of the jsi-rs cargo workspace, so its build output lands in
the workspace's `target/` directory (`../../../target` from the crate).

```bash
cd native/jsi-demo-module

# iOS simulator
cargo build --target aarch64-apple-ios-sim
cp ../../../target/aarch64-apple-ios-sim/debug/libjsi_demo_module.a libjsi-demo-module.a

# Android (arm64 emulator / device)
cargo ndk build -t arm64-v8a -P 24
mkdir -p android/arm64-v8a
cp ../../../target/aarch64-linux-android/debug/libjsi_demo_module.a android/arm64-v8a/
```

Both platforms link the Rust **static library** into the app's own native
build. The crate depends on `jsi` with `default-features = false`, which turns
off the `bundled-jsi` feature: React Native already ships jsi, and the Rust
library uses that copy instead of compiling a second one.

For iOS device builds, also build `aarch64-apple-ios` and either lipo the two
`.a` files together or update the podspec. For more Android ABIs, build each
target and copy it to `android/<ABI>/`.

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
# build only the ABIs you built the Rust library for
JAVA_HOME=<path-to-jdk-21> ./gradlew assembleDebug -PreactNativeArchitectures=arm64-v8a
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

On both platforms, `JsiDemoModule` is a TurboModule whose only job is to hand
the JS runtime to Rust, using React Native's official JSI bindings hook. React
Native runs that hook whenever the module is created, which happens again on
every reload, so the bindings are always installed into the live runtime.

- **iOS** — the pod (`JsiDemoModule.podspec`) vendors the Rust static library
  and compiles [`JsiDemoModule.mm`](native/jsi-demo-module/ios/JsiDemoModule.mm),
  which implements `RCTTurboModuleWithJSIBindings`. Its
  `installJSIBindingsWithRuntime:callInvoker:` calls the Rust
  `jsi_demo_install` entry point.
- **Android** — [`JsiDemoModule.kt`](android/app/src/main/java/com/jsidemo/JsiDemoModule.kt)
  implements `TurboModuleWithJSIBindings`. Its `getBindingsInstaller()` is
  implemented in C++ ([`JsiDemoBindings.cpp`](android/app/src/main/jni/JsiDemoBindings.cpp)),
  because the installer (`BindingsInstallerHolder`) can only be created through
  fbjni. The app's [`CMakeLists.txt`](android/app/src/main/jni/CMakeLists.txt)
  links that file and the Rust static library into `libappmodules.so`.
- **JS** — `App.tsx` calls `TurboModuleRegistry.get('JsiDemoModule')` once,
  which creates the module and installs the `RustDemo` global.

[cargo-ndk]: https://github.com/bbqsrc/cargo-ndk
