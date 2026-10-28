# RustTurbo platform carrier

One-time-per-app native glue that hands every new JS runtime to the Rust
aggregate library. Copy these files into your app, point them at your
`rust_turbo_install` (provided by your app's Rust aggregate crate — see
`example/native/rust-app`), and never touch platform code again when adding
Rust TurboModules.

## iOS

1. Copy `ios/RustTurboModule.mm` into your iOS folder (e.g. next to your
   Podfile or inside your module pod).
2. Link the Rust static library from your app's podspec:
   ```ruby
   s.vendored_libraries = "librust-app.a"   # built by cargo, see below
   s.dependency 'React-callinvoker'
   ```
3. Build the aggregate library and place the `.a` next to the podspec:
   ```bash
   cargo build --release -p rust-app --target aarch64-apple-ios-sim
   cp target/aarch64-apple-ios-sim/release/librust_app.a <pod-root>/librust-app.a
   ```
   (For device builds use `--target aarch64-apple-ios`, and lipo the two
   archives together if you need a universal binary.)

React Native calls `installJSIBindingsWithRuntime:callInvoker:` when the
`RustTurbo` TurboModule is first requested from JS — which the generated
TypeScript bindings do automatically (see the `bootstrap()` function in
`generated/rust-turbo/*.ts`).

## Android

1. Copy `android/RustTurboModule.kt` and `android/RustTurboPackage.kt` into
   your app's package, and `android/RustTurboBindings.cpp` into your JNI
   folder.
2. Register the package in `MainApplication.kt` (`packages.add(RustTurboPackage())`).
3. Build the aggregate static library with cargo-ndk:
   ```bash
   cargo ndk --target arm64-v8a --platform 24 --release build -p rust-app
   mkdir -p <app>/src/main/rust/arm64-v8a
   cp target/aarch64-linux-android/release/librust_app.a \
     <app>/src/main/rust/arm64-v8a/
   ```
4. Merge `android/CMakeLists-snippet` into your app's `CMakeLists.txt` (or
   use it as the whole file if you don't have one) so `libappmodules.so`
   links `librust_app.a`. The same static-library model as iOS keeps the
   cxx bridge's C++ side available to the linker (a Rust cdylib would drop
   those unreferenced objects).

## Adding modules

Adding a Rust TurboModule never touches platform code again:

1. Add the module's crate as a dependency of your aggregate `rust-app` crate
2. Add its force anchor to `FORCE_LINK` in the aggregate crate (required on
   iOS; harmless elsewhere): `static FORCE_LINK: [extern "C" fn(); 2] =
   [foo::__jsi_turbo_force_foo, bar::__jsi_turbo_force_bar];`
3. `import { getFoo } from '<...>/generated/rust-turbo/Foo'` from JS
