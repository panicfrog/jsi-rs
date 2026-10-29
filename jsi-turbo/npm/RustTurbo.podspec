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

  s.script_phases = [
    {
      :name => 'RustTurboBuild',
      :script => '"${PODS_TARGET_SRCROOT}/scripts/build-ios.sh"',
      :execution_position => :before_compile,
      :output_files => ["${PODS_TARGET_SRCROOT}/librust-app.a"],
      :show_env_vars_in_log => false
    }
  ]

  install_modules_dependencies(s)
end
