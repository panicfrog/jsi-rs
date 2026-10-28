# Builds libappmodules.so (React Native's standard app native library) plus the
# JSI bindings glue for the Rust module. Based on
# ReactAndroid/cmake-utils/default-app-setup/CMakeLists.txt.

cmake_minimum_required(VERSION 3.13)
project(appmodules)

include(${REACT_ANDROID_DIR}/cmake-utils/ReactNative-application.cmake)

# The Rust module, built by `cargo ndk` into jniLibs (see example/README.md).
# It has no SONAME, so IMPORTED_NO_SONAME makes the linker record it as
# `libjsi_demo_module.so` instead of a build machine path.
add_library(jsi_demo_module SHARED IMPORTED)
set_target_properties(jsi_demo_module PROPERTIES
  IMPORTED_LOCATION ${CMAKE_CURRENT_SOURCE_DIR}/../jniLibs/${ANDROID_ABI}/libjsi_demo_module.so
  IMPORTED_NO_SONAME TRUE)

target_link_libraries(${CMAKE_PROJECT_NAME} jsi_demo_module)
