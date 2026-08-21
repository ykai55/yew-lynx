#ifndef LYNX_NATIVE_APPLICATION_H_
#define LYNX_NATIVE_APPLICATION_H_

#include <stdint.h>

#include "lynx_native_renderer.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Both framework static libraries use bridge NodeId 1 for the native root. */
#define LYNX_ELEMENT_BRIDGE_NATIVE_ROOT_ID 1u

typedef uint32_t LynxElementBridgeSession;

typedef const LynxNativeRendererApiV1* (*LynxNativeRendererGetApiFn)(
    uint32_t requested_version);

typedef struct LynxElementBridgeNativeMountResult {
  LynxNativeRendererStatus status;
  LynxElementBridgeSession session;
} LynxElementBridgeNativeMountResult;

typedef struct LynxElementBridgeNativeDestroyResult {
  LynxNativeRendererStatus status;
  /* One only when the owner-thread call consumed the supplied session token. */
  uint32_t consumed;
} LynxElementBridgeNativeDestroyResult;

LynxElementBridgeNativeMountResult lynx_element_bridge_native_mount(
    LynxNativeRendererGetApiFn get_api,
    LynxNativeHostHandle host);
LynxElementBridgeNativeDestroyResult
lynx_element_bridge_native_destroy_session(LynxElementBridgeSession session);
/* Emergency teardown only; no application teardown mutations are applied. */
LynxElementBridgeNativeDestroyResult
lynx_element_bridge_native_abandon_session(LynxElementBridgeSession session);

/* Static storage identifying the linked backend: "yew" or "dioxus". */
const char* lynx_element_bridge_backend(void);
const char* lynx_element_bridge_backend_marker(void);

/*
 * Sessions are nonzero opaque 32-bit values scoped to the mounting thread.
 * A wrong-thread destroy returns consumed=0. Callers clear their token whenever
 * consumed=1, including status failures, and may use abandon only after an
 * unconsumed normal-destroy failure.
 */

#ifdef __cplusplus
}  /* extern "C" */
#endif

#endif  /* LYNX_NATIVE_APPLICATION_H_ */
