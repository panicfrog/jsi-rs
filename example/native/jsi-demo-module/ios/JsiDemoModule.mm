// A TurboModule whose only job is to install the Rust JSI bindings.
//
// React Native calls installJSIBindingsWithRuntime:callInvoker: when the
// module is first requested from JS (TurboModuleRegistry.get), handing it the
// jsi::Runtime directly. A new runtime is created on every reload, and the
// module is re-created with it, so the bindings are always installed into the
// live runtime before JS can use them.

#import <React/RCTBridgeModule.h>
#import <ReactCommon/RCTTurboModule.h>
#import <ReactCommon/RCTTurboModuleWithJSIBindings.h>

// implemented by the Rust library (jsi-demo-module/src/lib.rs)
extern "C" bool jsi_demo_install(void *runtime);

@interface JsiDemoModule : NSObject <RCTBridgeModule, RCTTurboModule, RCTTurboModuleWithJSIBindings>
@end

@implementation JsiDemoModule

RCT_EXPORT_MODULE(JsiDemoModule)

- (std::shared_ptr<facebook::react::TurboModule>)getTurboModule:
    (const facebook::react::ObjCTurboModule::InitParams &)params
{
  // no JS-callable methods; the module exists to receive the runtime
  return std::make_shared<facebook::react::ObjCTurboModule>(params);
}

- (void)installJSIBindingsWithRuntime:(facebook::jsi::Runtime &)runtime
                          callInvoker:(const std::shared_ptr<facebook::react::CallInvoker> &)callInvoker
{
  jsi_demo_install(&runtime);
}

@end
