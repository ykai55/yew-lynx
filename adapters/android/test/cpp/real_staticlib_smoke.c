#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "lynx_native_application.h"

#ifndef EXPECTED_BACKEND
#error "EXPECTED_BACKEND must identify the linked static library"
#endif
#ifndef EXPECTED_BACKEND_MARKER
#error "EXPECTED_BACKEND_MARKER must identify the linked static library"
#endif

static LynxNativeRendererHandle active_renderer;
static LynxNativeNodeHandle next_node = 2;

static LynxNativeRendererStatus acquire_renderer(
    LynxNativeHostHandle host,
    const LynxNativeRendererCallbacksV1* callbacks,
    LynxNativeRendererHandle* renderer) {
  if (host != 1 || callbacks == NULL || callbacks->on_event == NULL
      || callbacks->on_timer == NULL || renderer == NULL || active_renderer != 0) {
    return LYNX_NATIVE_RENDERER_STATUS_INVALID_ARGUMENT;
  }
  active_renderer = 1;
  *renderer = active_renderer;
  return LYNX_NATIVE_RENDERER_STATUS_OK;
}

static LynxNativeRendererStatus release_renderer(LynxNativeRendererHandle renderer) {
  if (renderer == 0 || renderer != active_renderer) {
    return LYNX_NATIVE_RENDERER_STATUS_INVALID_SESSION;
  }
  active_renderer = 0;
  return LYNX_NATIVE_RENDERER_STATUS_OK;
}

static LynxNativeRendererStatus get_root(LynxNativeRendererHandle renderer,
                                        LynxNativeNodeHandle* root) {
  if (renderer != active_renderer || root == NULL) {
    return LYNX_NATIVE_RENDERER_STATUS_INVALID_ARGUMENT;
  }
  *root = 1;
  return LYNX_NATIVE_RENDERER_STATUS_OK;
}

static LynxNativeRendererStatus create_node(LynxNativeRendererHandle renderer,
                                            LynxNativeUtf8 value,
                                            LynxNativeNodeHandle* node) {
  if (renderer != active_renderer || value.data == NULL || node == NULL) {
    return LYNX_NATIVE_RENDERER_STATUS_INVALID_ARGUMENT;
  }
  *node = next_node++;
  return LYNX_NATIVE_RENDERER_STATUS_OK;
}

static LynxNativeRendererStatus set_raw_text(LynxNativeRendererHandle renderer,
                                             LynxNativeNodeHandle node,
                                             LynxNativeUtf8 text) {
  return renderer == active_renderer && node != 0 && text.data != NULL
      ? LYNX_NATIVE_RENDERER_STATUS_OK
      : LYNX_NATIVE_RENDERER_STATUS_INVALID_ARGUMENT;
}

static LynxNativeRendererStatus set_attribute(LynxNativeRendererHandle renderer,
                                              LynxNativeNodeHandle node,
                                              LynxNativeUtf8 name,
                                              LynxNativeUtf8 value) {
  (void)value;
  return renderer == active_renderer && node != 0 && name.data != NULL
      ? LYNX_NATIVE_RENDERER_STATUS_OK
      : LYNX_NATIVE_RENDERER_STATUS_INVALID_ARGUMENT;
}

static LynxNativeRendererStatus mutate_tree(LynxNativeRendererHandle renderer,
                                            LynxNativeNodeHandle parent,
                                            LynxNativeNodeHandle child,
                                            LynxNativeNodeHandle reference) {
  (void)reference;
  return renderer == active_renderer && parent != 0 && child != 0
      ? LYNX_NATIVE_RENDERER_STATUS_OK
      : LYNX_NATIVE_RENDERER_STATUS_INVALID_ARGUMENT;
}

static LynxNativeRendererStatus remove_child(LynxNativeRendererHandle renderer,
                                             LynxNativeNodeHandle parent,
                                             LynxNativeNodeHandle child) {
  return mutate_tree(renderer, parent, child, 0);
}

static LynxNativeRendererStatus destroy_node(LynxNativeRendererHandle renderer,
                                             LynxNativeNodeHandle node) {
  return renderer == active_renderer && node != 0
      ? LYNX_NATIVE_RENDERER_STATUS_OK
      : LYNX_NATIVE_RENDERER_STATUS_INVALID_ARGUMENT;
}

static LynxNativeRendererStatus add_listener(
    LynxNativeRendererHandle renderer,
    LynxNativeNodeHandle node,
    LynxNativeListenerHandle listener,
    LynxNativeCallbackHandle callback,
    LynxNativeUtf8 name) {
  return renderer == active_renderer && node != 0 && listener != 0 && callback != 0
          && name.data != NULL
      ? LYNX_NATIVE_RENDERER_STATUS_OK
      : LYNX_NATIVE_RENDERER_STATUS_INVALID_ARGUMENT;
}

static LynxNativeRendererStatus remove_listener(
    LynxNativeRendererHandle renderer,
    LynxNativeNodeHandle node,
    LynxNativeListenerHandle listener,
    LynxNativeCallbackHandle callback,
    LynxNativeUtf8 name) {
  return add_listener(renderer, node, listener, callback, name);
}

static LynxNativeRendererStatus flush(LynxNativeRendererHandle renderer) {
  return renderer == active_renderer ? LYNX_NATIVE_RENDERER_STATUS_OK
                                     : LYNX_NATIVE_RENDERER_STATUS_INVALID_SESSION;
}

static LynxNativeRendererStatus create_timer(LynxNativeRendererHandle renderer,
                                             uint64_t delay_millis,
                                             uint32_t repeating,
                                             LynxNativeCallbackHandle callback,
                                             LynxNativeTimerHandle* timer) {
  (void)delay_millis;
  (void)repeating;
  if (renderer != active_renderer || callback == 0 || timer == NULL) {
    return LYNX_NATIVE_RENDERER_STATUS_INVALID_ARGUMENT;
  }
  *timer = 1;
  return LYNX_NATIVE_RENDERER_STATUS_OK;
}

static LynxNativeRendererStatus cancel_timer(LynxNativeRendererHandle renderer,
                                             LynxNativeTimerHandle timer) {
  return renderer == active_renderer && timer != 0
      ? LYNX_NATIVE_RENDERER_STATUS_OK
      : LYNX_NATIVE_RENDERER_STATUS_INVALID_ARGUMENT;
}

static const LynxNativeRendererApiV1 native_api = {
    LYNX_NATIVE_RENDERER_ABI_VERSION,
    sizeof(LynxNativeRendererApiV1),
    acquire_renderer,
    release_renderer,
    get_root,
    create_node,
    create_node,
    set_raw_text,
    set_attribute,
    mutate_tree,
    remove_child,
    destroy_node,
    add_listener,
    remove_listener,
    flush,
    create_timer,
    cancel_timer,
};

static const LynxNativeRendererApiV1* get_native_api(uint32_t version) {
  return version == LYNX_NATIVE_RENDERER_ABI_VERSION ? &native_api : NULL;
}

int main(void) {
  assert(strcmp(lynx_element_bridge_backend(), EXPECTED_BACKEND) == 0);
  assert(strcmp(lynx_element_bridge_backend_marker(),
                 EXPECTED_BACKEND_MARKER) == 0);

  LynxElementBridgeNativeMountResult native_mounted =
      lynx_element_bridge_native_mount(get_native_api, 1);
  assert(native_mounted.status == LYNX_NATIVE_RENDERER_STATUS_OK);
  assert(native_mounted.session != 0);
  LynxElementBridgeNativeDestroyResult native_abandoned =
      lynx_element_bridge_native_abandon_session(native_mounted.session);
  assert(native_abandoned.status == LYNX_NATIVE_RENDERER_STATUS_OK);
  assert(native_abandoned.consumed == 1);
  native_abandoned =
      lynx_element_bridge_native_abandon_session(native_mounted.session);
  assert(native_abandoned.status == LYNX_NATIVE_RENDERER_STATUS_INVALID_SESSION);
  assert(native_abandoned.consumed == 0);

  native_mounted = lynx_element_bridge_native_mount(get_native_api, 1);
  assert(native_mounted.status == LYNX_NATIVE_RENDERER_STATUS_OK);
  assert(native_mounted.session != 0);
  LynxElementBridgeNativeDestroyResult native_destroyed =
      lynx_element_bridge_native_destroy_session(native_mounted.session);
  assert(native_destroyed.status == LYNX_NATIVE_RENDERER_STATUS_OK);
  assert(native_destroyed.consumed == 1);
  native_destroyed =
      lynx_element_bridge_native_destroy_session(native_mounted.session);
  assert(native_destroyed.status == LYNX_NATIVE_RENDERER_STATUS_INVALID_SESSION);
  assert(native_destroyed.consumed == 0);

  return EXIT_SUCCESS;
}
