#include "lynx_native_application.h"

namespace {

constexpr LynxElementBridgeSession kSession = 51;
constexpr LynxNativeHostHandle kHost = UINT64_C(0x100000001);
bool g_mounted = false;

}  // namespace

extern "C" const char* lynx_element_bridge_backend(void) {
  return "mock";
}

extern "C" const char* lynx_element_bridge_backend_marker(void) {
  return "lynx-element-bridge-backend:mock";
}

extern "C" LynxElementBridgeNativeMountResult
lynx_element_bridge_native_mount(LynxNativeRendererGetApiFn get_api,
                                 LynxNativeHostHandle host) {
  if (host >= 2 && host <= 10) {
    return {static_cast<LynxNativeRendererStatus>(host), 0};
  }
  if (host == 11) {
    return {99, 0};
  }
  if (g_mounted || host != kHost || get_api == nullptr ||
      get_api(LYNX_NATIVE_RENDERER_ABI_VERSION) == nullptr) {
    return {LYNX_NATIVE_RENDERER_STATUS_INVALID_ARGUMENT, 0};
  }
  g_mounted = true;
  return {LYNX_NATIVE_RENDERER_STATUS_OK, kSession};
}

extern "C" LynxElementBridgeNativeDestroyResult
lynx_element_bridge_native_destroy_session(LynxElementBridgeSession session) {
  if (!g_mounted || session != kSession) {
    return {LYNX_NATIVE_RENDERER_STATUS_INVALID_SESSION, 0};
  }
  g_mounted = false;
  return {LYNX_NATIVE_RENDERER_STATUS_OK, 1};
}

extern "C" LynxElementBridgeNativeDestroyResult
lynx_element_bridge_native_abandon_session(LynxElementBridgeSession session) {
  if (!g_mounted || session != kSession) {
    return {LYNX_NATIVE_RENDERER_STATUS_INVALID_SESSION, 0};
  }
  g_mounted = false;
  return {LYNX_NATIVE_RENDERER_STATUS_OK, 1};
}
