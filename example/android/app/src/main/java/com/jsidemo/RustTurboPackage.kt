// RustTurbo carrier for Android (package half). Copy once per app and
// register it in MainApplication.kt: packages.add(RustTurboPackage()).

package com.jsidemo

import com.facebook.react.BaseReactPackage
import com.facebook.react.bridge.NativeModule
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.module.model.ReactModuleInfo
import com.facebook.react.module.model.ReactModuleInfoProvider

class RustTurboPackage : BaseReactPackage() {

  override fun getModule(name: String, reactContext: ReactApplicationContext): NativeModule? =
      if (name == RustTurboModule.NAME) RustTurboModule(reactContext) else null

  override fun getReactModuleInfoProvider() = ReactModuleInfoProvider {
    mapOf(
        RustTurboModule.NAME to
            ReactModuleInfo(
                name = RustTurboModule.NAME,
                className = RustTurboModule::class.java.name,
                canOverrideExistingModule = false,
                needsEagerInit = false,
                isCxxModule = false,
                isTurboModule = true,
            ))
  }
}
