#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "yew_lynx.h"

static int response_contains(YewLynxBuffer response, const char* text) {
  const size_t text_len = strlen(text);
  if (response.data == NULL || text_len > response.len) {
    return 0;
  }
  for (size_t index = 0; index + text_len <= response.len; ++index) {
    if (memcmp(response.data + index, text, text_len) == 0) {
      return 1;
    }
  }
  return 0;
}

int main(void) {
  static const uint8_t root_id[] = {'1'};
  YewLynxMountResult mounted = yew_lynx_mount(root_id, sizeof(root_id));
  assert(mounted.session != 0);
  assert(response_contains(mounted.response, "\"ok\":true"));
  assert(response_contains(mounted.response, "\"op\":\"flush\""));
  yew_lynx_buffer_free(mounted.response);

  YewLynxDestroyResult destroyed = yew_lynx_destroy(mounted.session);
  assert(destroyed.consumed == 1);
  assert(response_contains(destroyed.response, "\"ok\":true"));
  assert(response_contains(destroyed.response, "\"op\":\"flush\""));
  yew_lynx_buffer_free(destroyed.response);
  return EXIT_SUCCESS;
}
