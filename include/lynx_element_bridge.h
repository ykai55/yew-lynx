#ifndef LYNX_ELEMENT_BRIDGE_H_
#define LYNX_ELEMENT_BRIDGE_H_

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define LYNX_ELEMENT_BRIDGE_ABI_VERSION 2u
#define LYNX_ELEMENT_BRIDGE_PROTOCOL_VERSION 2u

typedef uint32_t LynxElementBridgeSession;

typedef struct LynxElementBridgeBuffer {
  uint8_t* data;
  size_t len;
} LynxElementBridgeBuffer;

typedef struct LynxElementBridgeMountResult {
  LynxElementBridgeSession session;
  LynxElementBridgeBuffer response;
} LynxElementBridgeMountResult;

typedef struct LynxElementBridgeDestroyResult {
  /* One only when the owner-thread call consumed the supplied session token. */
  uint32_t consumed;
  LynxElementBridgeBuffer response;
} LynxElementBridgeDestroyResult;

LynxElementBridgeMountResult lynx_element_bridge_mount(uint32_t root_id);
LynxElementBridgeBuffer lynx_element_bridge_dispatch_event(
    LynxElementBridgeSession session,
    const uint8_t* event,
    size_t event_len);
LynxElementBridgeBuffer lynx_element_bridge_complete_batch(
    LynxElementBridgeSession session,
    const uint8_t* response,
    size_t response_len);
LynxElementBridgeDestroyResult lynx_element_bridge_destroy_session(
    LynxElementBridgeSession session);
void lynx_element_bridge_buffer_free(LynxElementBridgeBuffer buffer);

/* Static storage identifying the linked backend: "yew" or "dioxus". */
const char* lynx_element_bridge_backend(void);
const char* lynx_element_bridge_backend_marker(void);

/*
 * IDs are nonzero opaque 32-bit integers scoped to one owner thread and
 * session. Inputs and responses are FlatBuffers v2 `LEB2` envelopes. Returned
 * buffers must be freed exactly once. A wrong-thread destroy returns
 * consumed=0; callers clear the token whenever consumed=1, even on failure.
 */

#ifdef __cplusplus
}  /* extern "C" */
#endif

#endif  /* LYNX_ELEMENT_BRIDGE_H_ */
