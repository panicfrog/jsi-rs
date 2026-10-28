# Builds libappmodules.so (React Native's standard app native library) plus the
# JSI bindings glue for the Rust module. Based on
# ReactAndroid/cmake-utils/default-app-setup/CMakeLists.txt.

cmake_minimum_required(VERSION 3.13)
project(appmodules)

include(${REACT_ANDROID_DIR}/cmake-utils/ReactNative-application.cmake)

# The Rust aggregate library (every Rust module + the jsi-turbo runtime + the
# cxx bridge C++ side), built by `cargo ndk ... build -p rust-app` (see
# example/README.md). Same model as iOS: the platform linker pulls in the
# archive members the glue references.
add_library(rust_app STATIC IMPORTED)
set_target_properties(rust_app PROPERTIES
  IMPORTED_LOCATION ${CMAKE_CURRENT_SOURCE_DIR}/../rust/${ANDROID_ABI}/librust_app.a)

target_link_libraries(${CMAKE_PROJECT_NAME} rust_app)
