#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "yew_lynx.h"

static int is_protocol_v2(YewLynxBuffer response) {
  return response.data != NULL && response.len >= 8
      && memcmp(response.data + 4, "LEB2", 4) == 0;
}

int main(void) {
  YewLynxMountResult mounted = yew_lynx_mount(1);
  assert(mounted.session != 0);
  assert(is_protocol_v2(mounted.response));
  yew_lynx_buffer_free(mounted.response);

  YewLynxDestroyResult destroyed = yew_lynx_destroy(mounted.session);
  assert(destroyed.consumed == 1);
  assert(is_protocol_v2(destroyed.response));
  yew_lynx_buffer_free(destroyed.response);
  return EXIT_SUCCESS;
}
