cmake_minimum_required(VERSION 3.22.1)

project(yew_lynx_android LANGUAGES C CXX)

set(YEW_LYNX_RUST_LIB_DIR "" CACHE PATH
    "Directory containing ABI subdirectories with libyew_lynx_counter.a")
if(NOT YEW_LYNX_RUST_LIB_DIR)
  message(FATAL_ERROR "YEW_LYNX_RUST_LIB_DIR must point to the Rust archive directory")
endif()
if(NOT ANDROID_ABI)
  message(FATAL_ERROR "The yew-lynx JNI bridge must be built with the Android NDK")
endif()

add_library(yew_lynx_counter STATIC IMPORTED)
set_target_properties(yew_lynx_counter PROPERTIES
  IMPORTED_LOCATION
      "${YEW_LYNX_RUST_LIB_DIR}/${ANDROID_ABI}/libyew_lynx_counter.a")

add_library(yew_lynx_bridge SHARED
  src/main/cpp/yew_lynx_jni.cc)

target_include_directories(yew_lynx_bridge PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/../../examples/counter/include")
target_compile_features(yew_lynx_bridge PRIVATE cxx_std_17)
target_compile_options(yew_lynx_bridge PRIVATE
  -Wall
  -Wextra
  -Werror)
# Rust 1.85 reports these for aarch64-linux-android via
# `cargo rustc -- --print native-static-libs`; keep their link order.
target_link_libraries(yew_lynx_bridge PRIVATE
  yew_lynx_counter
  dl
  log
  unwind
  dl
  m
  c)
