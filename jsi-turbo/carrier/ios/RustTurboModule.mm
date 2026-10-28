// RustTurbo carrier for iOS: a TurboModule whose only job is to hand the
// jsi::Runtime (and the CallInvoker) to the Rust aggregate library on every
// runtime creation. Copy once per app — adding Rust TurboModules later does
// not touch this file.
//
// The entrypoint is implemented by the app's Rust aggregate crate
// (see jsi-turbo/carrier/README.md); its signature matches the cxx bridge
// function `rust_turbo_install(facebook::jsi::Runtime&, std::shared_ptr<facebook::react::CallInvoker>)`.

#import <React/RCTBridgeModule.h>
#import <ReactCommon/RCTTurboModule.h>
#import <ReactCommon/RCTTurboModuleWithJSIBindings.h>

#include <memory>

namespace facebook {
  namespace jsi { class Runtime; }
  namespace react { class CallInvoker; }
}

bool rust_turbo_install(facebook::jsi::Runtime &runtime,
                        std::shared_ptr<facebook::react::CallInvoker> invoker);

@interface RustTurboModule : NSObject <RCTBridgeModule, RCTTurboModule, RCTTurboModuleWithJSIBindings>
@end

@implementation RustTurboModule

RCT_EXPORT_MODULE(RustTurbo)

- (std::shared_ptr<facebook::react::TurboModule>)getTurboModule:
    (const facebook::react::ObjCTurboModule::InitParams &)params
{
  // no JS-callable methods; the module exists to receive the runtime
  return std::make_shared<facebook::react::ObjCTurboModule>(params);
}

- (void)installJSIBindingsWithRuntime:(facebook::jsi::Runtime &)runtime
                          callInvoker:(const std::shared_ptr<facebook::react::CallInvoker> &)callInvoker
{
  rust_turbo_install(runtime, callInvoker);
}

@end
