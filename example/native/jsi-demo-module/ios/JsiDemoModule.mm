#import <React/RCTBridgeModule.h>
#import <React/RCTBridge+Private.h>

// implemented by both RCTCxxBridge (bridge mode) and RCTBridgeProxy
// (bridgeless mode); both return the jsi::Runtime pointer as void *
@protocol JsiRuntimeProvider <NSObject>
- (void *)runtime;
@end

// implemented by the Rust static library (jsi-demo-module/src/lib.rs)
extern "C" bool jsi_demo_install(void *runtime);

@interface JsiDemoModule : NSObject <RCTBridgeModule>
@end

@implementation JsiDemoModule

RCT_EXPORT_MODULE(JsiDemoModule)

RCT_EXPORT_BLOCKING_SYNCHRONOUS_METHOD(install)
{
  RCTBridge *bridge = [RCTBridge currentBridge];
  if (bridge == nil) {
    return @(NO);
  }

  id<JsiRuntimeProvider> provider = (id<JsiRuntimeProvider>)bridge;
  void *runtime = [provider runtime];
  if (runtime == NULL) {
    return @(NO);
  }

  return @(jsi_demo_install(runtime));
}

@end
