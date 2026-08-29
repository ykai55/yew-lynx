cmake_minimum_required(VERSION 3.22.1)

project(lynx_element_bridge_android LANGUAGES C CXX)

set(LYNX_ELEMENT_BRIDGE_RUST_LIB_DIR "" CACHE PATH
    "Backend-specific directory containing ABI subdirectories")
set(LYNX_ELEMENT_BRIDGE_BACKEND "" CACHE STRING "Selected Rust backend")
set(LYNX_ELEMENT_BRIDGE_LIBRARY_NAME "lynx_element_bridge" CACHE STRING
    "JNI shared library target name")
set(LYNX_ELEMENT_BRIDGE_JNI_CLASS
    "Java_com_lynx_elementbridge_LynxNativeRendererHost_" CACHE STRING
    "JNI class symbol prefix")
if(NOT LYNX_ELEMENT_BRIDGE_RUST_LIB_DIR)
  message(FATAL_ERROR
      "LYNX_ELEMENT_BRIDGE_RUST_LIB_DIR must point to the Rust archive directory")
endif()
if(NOT ANDROID_ABI)
  message(FATAL_ERROR "The Lynx Element Bridge JNI must be built with the Android NDK")
endif()
if(NOT LYNX_ELEMENT_BRIDGE_BACKEND MATCHES "^(yew|dioxus|wasm)$")
  message(FATAL_ERROR
      "LYNX_ELEMENT_BRIDGE_BACKEND must be yew, dioxus, or wasm")
endif()

add_library(lynx_element_bridge_backend STATIC IMPORTED)
set_target_properties(lynx_element_bridge_backend PROPERTIES
  IMPORTED_LOCATION
      "${LYNX_ELEMENT_BRIDGE_RUST_LIB_DIR}/${ANDROID_ABI}/liblynx_element_bridge_backend.a")

add_library(${LYNX_ELEMENT_BRIDGE_LIBRARY_NAME} SHARED
  src/main/cpp/lynx_element_bridge_jni.cc
  ../../include/lynx_native_application.h
  ../../include/lynx_native_renderer.h)

target_include_directories(${LYNX_ELEMENT_BRIDGE_LIBRARY_NAME} PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/../../include")
target_compile_features(${LYNX_ELEMENT_BRIDGE_LIBRARY_NAME} PRIVATE cxx_std_17)
target_compile_definitions(${LYNX_ELEMENT_BRIDGE_LIBRARY_NAME} PRIVATE
  LYNX_ELEMENT_BRIDGE_JNI_CLASS=${LYNX_ELEMENT_BRIDGE_JNI_CLASS})
if(LYNX_ELEMENT_BRIDGE_BACKEND STREQUAL "wasm")
  target_compile_definitions(${LYNX_ELEMENT_BRIDGE_LIBRARY_NAME} PRIVATE
    LYNX_ELEMENT_BRIDGE_WAMR=1
    LYNX_ELEMENT_BRIDGE_WAMR_BACKEND_NAME="wasm"
    LYNX_ELEMENT_BRIDGE_WAMR_BACKEND_MARKER="lynx-element-bridge-backend:wasm")
endif()
target_compile_options(${LYNX_ELEMENT_BRIDGE_LIBRARY_NAME} PRIVATE
  -Wall
  -Wextra
  -Werror)
set(jni_version_script
    "${CMAKE_CURRENT_BINARY_DIR}/${LYNX_ELEMENT_BRIDGE_LIBRARY_NAME}.map")
file(GENERATE OUTPUT "${jni_version_script}" CONTENT
  "{ global: ${LYNX_ELEMENT_BRIDGE_JNI_CLASS}*; local: *; };\n")
target_link_options(${LYNX_ELEMENT_BRIDGE_LIBRARY_NAME} PRIVATE
  "-Wl,--version-script=${jni_version_script}"
  -Wl,-Bsymbolic)
# Rust 1.85 reports these for aarch64-linux-android via
# `cargo rustc -- --print native-static-libs`; keep their link order.
target_link_libraries(${LYNX_ELEMENT_BRIDGE_LIBRARY_NAME} PRIVATE
  lynx_element_bridge_backend
  dl
  log
  unwind
  dl
  m
  c)
