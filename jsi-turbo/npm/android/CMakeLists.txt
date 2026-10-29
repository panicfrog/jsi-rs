# react-native-rust-turbo: builds librustturbo.so — the JNI glue that hands
# every JS runtime to the Rust aggregate library. librust_app.a is produced
# by the rustTurboBuild gradle task (cargo) before CMake runs and linked
# statically into this .so, so apps don't touch their own CMakeLists.

cmake_minimum_required(VERSION 3.13)
project(rustturbo)

# React Native 0.88 headers use C++20 (requires-constraints in react/bridging)
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# prefab packages resolved via the library's gradle dependencies
find_package(ReactAndroid REQUIRED CONFIG)
find_package(fbjni REQUIRED CONFIG)

# The Rust aggregate static library (one per ABI, built by the
# rustTurboBuild gradle task).
add_library(rust_app STATIC IMPORTED)
set_target_properties(rust_app PROPERTIES
  IMPORTED_LOCATION ${CMAKE_CURRENT_SOURCE_DIR}/src/main/rust/${ANDROID_ABI}/librust_app.a)

add_library(rustturbo SHARED
  src/main/jni/RustTurboBindings.cpp)

target_link_libraries(rustturbo
  rust_app
  ReactAndroid::jsi
  ReactAndroid::reactnative
  fbjni::fbjni)
