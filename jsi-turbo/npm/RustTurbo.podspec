require 'json'

package = JSON.parse(File.read(File.join(__dir__, 'package.json')))

Pod::Spec.new do |s|
  s.name         = "RustTurbo"
  s.version      = package['version']
  s.summary      = package['description']
  s.license      = package['license']
  s.authors      = { "jsi-rs" => "https://github.com/panicfrog/jsi-rs" }
  s.homepage     = package['repository']['url']
  s.source       = { :path => "." }
  s.prepare_command = 'bash scripts/stub.sh'
  s.platform     = :ios, "15.1"
  s.source_files = "ios/RustTurboModule.mm"

  # built by the script phase below (aggregating every installed
  # rust-turbo module package into one cargo build); a placeholder is
  # created at pod install time so the link settings are generated
  s.vendored_libraries = "librust-app.a"

  # No :output_files on the script phase — DELIBERATELY. With outputs
  # declared and no :input_files, Xcode's dependency analysis sees the
  # archive from a previous build and skips the phase entirely: cargo
  # never runs, and the app silently links the stale librust-app.a
  # (source changes to the module packages stop reaching the binary —
  # observed as the app running days-old code with only ld version
  # warnings in the log). Without the declaration the phase runs on
  # every build; cargo's own incremental cache makes a no-change build
  # a few seconds — the same trade RN's "Bundle React Native code and
  # images" phase makes.
  s.script_phases = [
    {
      :name => 'RustTurboBuild',
      :script => '"${PODS_TARGET_SRCROOT}/scripts/build-ios.sh"',
      :execution_position => :before_compile,
      :show_env_vars_in_log => false
    }
  ]

  install_modules_dependencies(s)
end
