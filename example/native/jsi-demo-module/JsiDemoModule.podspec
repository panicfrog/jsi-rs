Pod::Spec.new do |s|
  s.name         = "JsiDemoModule"
  s.version      = "0.77.3"
  s.summary      = "Demo React Native JSI module written in Rust with jsi-rs"
  s.license      = { :type => "MIT" }
  s.authors     = { "jsi-rs demo" => "demo@example.com" }
  s.homepage    = "https://github.com/laptou/jsi-rs"
  s.source      = { :path => "." }
  s.dependency "React-Core"
  s.platform     = :ios, "15.1"
  s.source_files = "ios/*.{h,m,mm}"
  # built by: cd native/jsi-demo-module && cargo build --target aarch64-apple-ios-sim
  s.vendored_libraries = "libjsi-demo-module.a"
  s.pod_target_xcconfig = {
    "CLANG_CXX_LANGUAGE_STANDARD" => "c++17",
    "DEFINES_MODULE" => "NO",
  }
end
