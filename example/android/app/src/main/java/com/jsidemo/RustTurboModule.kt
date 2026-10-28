// RustTurbo carrier for Android (module half). Copy once per app.
//
// React Native calls [getBindingsInstaller] when this module is first
// requested from JS (TurboModuleRegistry.get('RustTurbo') — which the
// generated TypeScript bindings do automatically) and runs the returned
// installer with the live jsi::Runtime + CallInvoker. The installer is
// created in C++ (RustTurboBindings.cpp) and calls rust_turbo_install in
// the Rust aggregate library (librust_app.so).

package com.jsidemo

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

  // implemented in RustTurboBindings.cpp (libappmodules.so)
  @DoNotStrip external override fun getBindingsInstaller(): BindingsInstallerHolder

  companion object {
    const val NAME = "RustTurbo"
  }
}
