#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "lynx_element_bridge.h"

#ifndef EXPECTED_BACKEND
#error "EXPECTED_BACKEND must identify the linked static library"
#endif
#ifndef EXPECTED_BACKEND_MARKER
#error "EXPECTED_BACKEND_MARKER must identify the linked static library"
#endif

static int is_protocol_v2(LynxElementBridgeBuffer response) {
  return response.data != NULL && response.len >= 8
      && memcmp(response.data + 4, "LEB2", 4) == 0;
}

int main(void) {
  assert(strcmp(lynx_element_bridge_backend(), EXPECTED_BACKEND) == 0);
  assert(strcmp(lynx_element_bridge_backend_marker(),
                EXPECTED_BACKEND_MARKER) == 0);
  LynxElementBridgeMountResult mounted = lynx_element_bridge_mount(1);
  assert(mounted.session != 0);
  assert(is_protocol_v2(mounted.response));
  lynx_element_bridge_buffer_free(mounted.response);

  LynxElementBridgeDestroyResult destroyed =
      lynx_element_bridge_destroy_session(mounted.session);
  assert(destroyed.consumed == 1);
  assert(is_protocol_v2(destroyed.response));
  lynx_element_bridge_buffer_free(destroyed.response);
  return EXIT_SUCCESS;
}
