// JNI glue for JsiDemoModule.getBindingsInstaller().
//
// React Native calls getBindingsInstaller() when the TurboModule is created
// and runs the returned installer with the live jsi::Runtime. A new runtime
// (and a new module instance) is created on every reload, so the Rust
// bindings are always installed into the current runtime.

#include <fbjni/fbjni.h>
#include <jsi/jsi.h>
#include <ReactCommon/BindingsInstallerHolder.h>
#include <ReactCommon/CallInvoker.h>

// implemented by the Rust library (native/jsi-demo-module/src/lib.rs)
extern "C" bool jsi_demo_install(void *runtime);

extern "C" JNIEXPORT jobject JNICALL
Java_com_jsidemo_JsiDemoModule_getBindingsInstaller(JNIEnv *, jobject) {
  using facebook::react::BindingsInstallerHolder;
  return BindingsInstallerHolder::newObjectCxxArgs(
             [](facebook::jsi::Runtime &runtime,
                const std::shared_ptr<facebook::react::CallInvoker> &) {
               jsi_demo_install(&runtime);
             })
      .release();
}
