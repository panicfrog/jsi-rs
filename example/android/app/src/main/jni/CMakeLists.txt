# Builds libappmodules.so (React Native's standard app native library) plus the
# JSI bindings glue for the Rust module. Based on
# ReactAndroid/cmake-utils/default-app-setup/CMakeLists.txt.

cmake_minimum_required(VERSION 3.13)
project(appmodules)

include(${REACT_ANDROID_DIR}/cmake-utils/ReactNative-application.cmake)

# The Rust module as a static library, built by `cargo ndk` and copied to
# native/jsi-demo-module/android/<ABI>/ (see example/README.md). It is linked
# into libappmodules.so, where React Native's jsi resolves its jsi symbols.
add_library(jsi_demo_module STATIC IMPORTED)
set_target_properties(jsi_demo_module PROPERTIES
  IMPORTED_LOCATION ${CMAKE_CURRENT_SOURCE_DIR}/../../../../../native/jsi-demo-module/android/${ANDROID_ABI}/libjsi_demo_module.a)

target_link_libraries(${CMAKE_PROJECT_NAME} jsi_demo_module)
