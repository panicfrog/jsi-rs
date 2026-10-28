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
  # the Rust aggregate library (all modules + jsi-turbo), built by:
  #   cargo build --release -p rust-app --target aarch64-apple-ios-sim
  #   cp target/.../librust_app.a librust-app.a   (see example/README.md)
  s.vendored_libraries = "librust-app.a"

  # pulls in React-Core, ReactCommon/turbomodule and the C++ settings that
  # TurboModules need
  install_modules_dependencies(s)
end
