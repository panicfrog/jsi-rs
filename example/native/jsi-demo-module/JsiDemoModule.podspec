Pod::Spec.new do |s|
  s.name         = "JsiDemoModule"
  s.version      = "0.88.1"
  s.summary      = "Demo React Native JSI module written in Rust with jsi-rs"
  s.license      = { :type => "MIT" }
  s.authors      = { "jsi-rs demo" => "demo@example.com" }
  s.homepage     = "https://github.com/laptou/jsi-rs"
  s.source       = { :path => "." }
  s.platform     = :ios, "15.1"
  s.source_files = "ios/*.{h,m,mm}"
  # built by: cargo build --target aarch64-apple-ios-sim (see example/README.md)
  s.vendored_libraries = "libjsi-demo-module.a"

  # pulls in React-Core, ReactCommon/turbomodule and the C++ settings that
  # TurboModules need
  install_modules_dependencies(s)
end
