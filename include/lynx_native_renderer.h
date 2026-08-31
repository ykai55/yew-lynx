#ifndef LYNX_NATIVE_RENDERER_H_
#define LYNX_NATIVE_RENDERER_H_

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define LYNX_NATIVE_RENDERER_ABI_VERSION 1u

typedef uint32_t LynxNativeRendererStatus;

enum {
  LYNX_NATIVE_RENDERER_STATUS_OK = 0,
  LYNX_NATIVE_RENDERER_STATUS_INVALID_ARGUMENT = 1,
  LYNX_NATIVE_RENDERER_STATUS_INVALID_SESSION = 2,
  LYNX_NATIVE_RENDERER_STATUS_WRONG_THREAD = 3,
  LYNX_NATIVE_RENDERER_STATUS_UNSUPPORTED = 4,
  LYNX_NATIVE_RENDERER_STATUS_INVALID_OWNERSHIP = 5,
  LYNX_NATIVE_RENDERER_STATUS_INVALID_LISTENER = 6,
  LYNX_NATIVE_RENDERER_STATUS_RESOURCE_EXHAUSTED = 7,
  LYNX_NATIVE_RENDERER_STATUS_HOST_ERROR = 8,
  LYNX_NATIVE_RENDERER_STATUS_PANIC = 9,
  LYNX_NATIVE_RENDERER_STATUS_INTERNAL_ERROR = 10,
};

typedef uint64_t LynxNativeHostHandle;
typedef uint64_t LynxNativeRendererHandle;
typedef uint32_t LynxNativeNodeHandle;
typedef uint32_t LynxNativeListenerHandle;
typedef uint32_t LynxNativeTimerHandle;
typedef uint32_t LynxNativeCallbackHandle;

/* Spans are borrowed for one call. UTF-8 spans must contain valid UTF-8. */
typedef struct LynxNativeUtf8 {
  const uint8_t* data;
  size_t len;
} LynxNativeUtf8;

typedef struct LynxNativeBytes {
  const uint8_t* data;
  size_t len;
} LynxNativeBytes;

typedef LynxNativeRendererStatus (*LynxNativeOnEvent)(
    void* context,
    LynxNativeRendererHandle renderer,
    LynxNativeListenerHandle listener,
    LynxNativeCallbackHandle callback,
    LynxNativeUtf8 name,
    LynxNativeUtf8 content_type,
    LynxNativeBytes payload);

typedef LynxNativeRendererStatus (*LynxNativeOnTimer)(
    void* context,
    LynxNativeRendererHandle renderer,
    LynxNativeTimerHandle timer,
    LynxNativeCallbackHandle callback);

typedef struct LynxNativeRendererCallbacksV1 {
  void* context;
  LynxNativeOnEvent on_event;
  LynxNativeOnTimer on_timer;
} LynxNativeRendererCallbacksV1;

typedef struct LynxNativeRendererApiV1 {
  uint32_t abi_version;
  size_t struct_size;

  /* acquire copies the callback table and retains only its context value. */
  LynxNativeRendererStatus (*acquire)(
      LynxNativeHostHandle host,
      const LynxNativeRendererCallbacksV1* callbacks,
      LynxNativeRendererHandle* renderer);
  LynxNativeRendererStatus (*release)(LynxNativeRendererHandle renderer);
  LynxNativeRendererStatus (*get_root)(LynxNativeRendererHandle renderer,
                                       LynxNativeNodeHandle* root);
  LynxNativeRendererStatus (*create_element)(
      LynxNativeRendererHandle renderer,
      LynxNativeUtf8 tag,
      LynxNativeNodeHandle* node);
  LynxNativeRendererStatus (*create_raw_text)(
      LynxNativeRendererHandle renderer,
      LynxNativeUtf8 text,
      LynxNativeNodeHandle* node);
  LynxNativeRendererStatus (*set_raw_text)(LynxNativeRendererHandle renderer,
                                           LynxNativeNodeHandle node,
                                           LynxNativeUtf8 text);
  /* A NULL value.data removes the attribute; non-NULL with len zero is empty. */
  LynxNativeRendererStatus (*set_attribute)(
      LynxNativeRendererHandle renderer,
      LynxNativeNodeHandle node,
      LynxNativeUtf8 name,
      LynxNativeUtf8 value);
  /* A zero reference appends child to parent. */
  LynxNativeRendererStatus (*insert_before)(
      LynxNativeRendererHandle renderer,
      LynxNativeNodeHandle parent,
      LynxNativeNodeHandle child,
      LynxNativeNodeHandle reference);
  LynxNativeRendererStatus (*remove_child)(LynxNativeRendererHandle renderer,
                                           LynxNativeNodeHandle parent,
                                           LynxNativeNodeHandle child);
  LynxNativeRendererStatus (*destroy_node)(LynxNativeRendererHandle renderer,
                                           LynxNativeNodeHandle node);
  LynxNativeRendererStatus (*add_event_listener)(
      LynxNativeRendererHandle renderer,
      LynxNativeNodeHandle node,
      LynxNativeListenerHandle listener,
      LynxNativeCallbackHandle callback,
      LynxNativeUtf8 name);
  LynxNativeRendererStatus (*remove_event_listener)(
      LynxNativeRendererHandle renderer,
      LynxNativeNodeHandle node,
      LynxNativeListenerHandle listener,
      LynxNativeCallbackHandle callback,
      LynxNativeUtf8 name);
  LynxNativeRendererStatus (*flush)(LynxNativeRendererHandle renderer);
  LynxNativeRendererStatus (*create_timer)(
      LynxNativeRendererHandle renderer,
      uint64_t delay_millis,
      uint32_t repeating,
      LynxNativeCallbackHandle callback,
      LynxNativeTimerHandle* timer);
  LynxNativeRendererStatus (*cancel_timer)(LynxNativeRendererHandle renderer,
                                           LynxNativeTimerHandle timer);
  /*
   * Imports one raw CSS fragment produced by this pinned Lynx encoder with
   * target SDK = current engine version, CSS rule/parser/selector/invalidation
   * enabled, and inline CSS variables disabled. No other compile profile is
   * supported.
   */
  LynxNativeRendererStatus (*import_style_sheet)(
      LynxNativeRendererHandle renderer,
      LynxNativeBytes fragment);
  /* Removes only stylesheets imported through this renderer handle. */
  LynxNativeRendererStatus (*clear_style_sheets)(
      LynxNativeRendererHandle renderer);
} LynxNativeRendererApiV1;

/*
 * Tables have static lifetime. Consumers copy only a supported prefix after
 * checking abi_version and struct_size. Compatible fields may be added only at
 * the tail. Unsupported versions return NULL.
 */
const LynxNativeRendererApiV1* lynx_native_renderer_get_api(
    uint32_t requested_version);

/*
 * Handles are opaque and nonzero except insert_before's optional reference.
 * Calls, callbacks, and release run synchronously on the acquiring thread.
 * release consumes a live renderer exactly once; stale releases are rejected.
 * No callback may run after its listener/timer or renderer is released.
 */

#ifdef __cplusplus
}  /* extern "C" */
#endif

#endif  /* LYNX_NATIVE_RENDERER_H_ */
