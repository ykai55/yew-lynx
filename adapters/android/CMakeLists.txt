cmake_minimum_required(VERSION 3.22.1)

project(lynx_element_bridge_android LANGUAGES C CXX)

set(LYNX_ELEMENT_BRIDGE_RUST_LIB_DIR "" CACHE PATH
    "Backend-specific directory containing ABI subdirectories")
set(LYNX_ELEMENT_BRIDGE_BACKEND "" CACHE STRING "Selected Rust backend")
if(NOT LYNX_ELEMENT_BRIDGE_RUST_LIB_DIR)
  message(FATAL_ERROR
      "LYNX_ELEMENT_BRIDGE_RUST_LIB_DIR must point to the Rust archive directory")
endif()
if(NOT ANDROID_ABI)
  message(FATAL_ERROR "The Lynx Element Bridge JNI must be built with the Android NDK")
endif()
if(NOT LYNX_ELEMENT_BRIDGE_BACKEND MATCHES "^(yew|dioxus)$")
  message(FATAL_ERROR "LYNX_ELEMENT_BRIDGE_BACKEND must be yew or dioxus")
endif()

add_library(lynx_element_bridge_backend STATIC IMPORTED)
set_target_properties(lynx_element_bridge_backend PROPERTIES
  IMPORTED_LOCATION
      "${LYNX_ELEMENT_BRIDGE_RUST_LIB_DIR}/${ANDROID_ABI}/liblynx_element_bridge_backend.a")

add_library(lynx_element_bridge SHARED
  src/main/cpp/lynx_element_bridge_jni.cc)

target_include_directories(lynx_element_bridge PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/../../include")
target_compile_features(lynx_element_bridge PRIVATE cxx_std_17)
target_compile_options(lynx_element_bridge PRIVATE
  -Wall
  -Wextra
  -Werror)
# Rust 1.85 reports these for aarch64-linux-android via
# `cargo rustc -- --print native-static-libs`; keep their link order.
target_link_libraries(lynx_element_bridge PRIVATE
  lynx_element_bridge_backend
  dl
  log
  unwind
  dl
  m
  c)
