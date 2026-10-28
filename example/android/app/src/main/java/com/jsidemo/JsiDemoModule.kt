package com.jsidemo

import com.facebook.proguard.annotations.DoNotStrip
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.ReactContextBaseJavaModule
import com.facebook.react.module.annotations.ReactModule
import com.facebook.react.turbomodule.core.interfaces.BindingsInstallerHolder
import com.facebook.react.turbomodule.core.interfaces.TurboModule
import com.facebook.react.turbomodule.core.interfaces.TurboModuleWithJSIBindings

/**
 * A TurboModule whose only job is to install the Rust JSI bindings.
 *
 * React Native calls [getBindingsInstaller] when the module is first requested
 * from JS (TurboModuleRegistry.get) and runs the returned installer with the
 * live jsi::Runtime. The installer is created in C++
 * (src/main/jni/JsiDemoBindings.cpp) and calls into the Rust library.
 */
@ReactModule(name = JsiDemoModule.NAME)
class JsiDemoModule(reactContext: ReactApplicationContext) :
    ReactContextBaseJavaModule(reactContext), TurboModule, TurboModuleWithJSIBindings {

  override fun getName() = NAME

  // implemented in src/main/jni/JsiDemoBindings.cpp (libappmodules.so)
  @DoNotStrip external override fun getBindingsInstaller(): BindingsInstallerHolder

  companion object {
    const val NAME = "JsiDemoModule"
  }
}
