package com.jsidemo

import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.ReactContextBaseJavaModule
import com.facebook.react.bridge.ReactMethod

class JsiDemoModule(reactContext: ReactApplicationContext) :
    ReactContextBaseJavaModule(reactContext) {

  companion object {
    init {
      // loads libjsi_demo_module.so from jniLibs/arm64-v8a
      System.loadLibrary("jsi_demo_module")
    }
  }

  // implemented by the Rust library (native/jsi-demo-module/src/lib.rs)
  private external fun init(runtimePtr: Long)

  override fun getName() = "JsiDemoModule"

  @ReactMethod(isBlockingSynchronousMethod = true)
  fun install(): Boolean {
    val runtimePtr = reactApplicationContext.javaScriptContextHolder?.get() ?: 0L
    if (runtimePtr != 0L) {
      init(runtimePtr)
      return true
    }
    return false
  }
}
