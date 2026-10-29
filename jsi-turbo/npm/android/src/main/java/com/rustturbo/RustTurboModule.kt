// RustTurbo carrier for Android (module half), shipped inside the
// react-native-rust-turbo library module.
//
// React Native calls [getBindingsInstaller] when this module is first
// requested from JS (TurboModuleRegistry.get('RustTurbo') — which the
// generated TypeScript bindings do automatically) and runs the returned
// installer with the live jsi::Runtime + CallInvoker. The installer is
// created in C++ (src/main/jni/RustTurboBindings.cpp) and calls
// rust_turbo_install in the Rust aggregate library linked into
// librustturbo.so.

package com.rustturbo

import com.facebook.proguard.annotations.DoNotStrip
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.ReactContextBaseJavaModule
import com.facebook.react.module.annotations.ReactModule
import com.facebook.react.turbomodule.core.interfaces.BindingsInstallerHolder
import com.facebook.react.turbomodule.core.interfaces.TurboModule
import com.facebook.react.turbomodule.core.interfaces.TurboModuleWithJSIBindings

@ReactModule(name = RustTurboModule.NAME)
class RustTurboModule(reactContext: ReactApplicationContext) :
    ReactContextBaseJavaModule(reactContext), TurboModule, TurboModuleWithJSIBindings {

  override fun getName() = NAME

  // implemented in src/main/jni/RustTurboBindings.cpp (librustturbo.so)
  @DoNotStrip external override fun getBindingsInstaller(): BindingsInstallerHolder

  companion object {
    const val NAME = "RustTurbo"

    init {
      // librustturbo.so carries the JNI glue + the statically linked Rust
      // aggregate library; load it before the external method is resolved
      System.loadLibrary("rustturbo")
    }
  }
}
