#ifndef YEW_LYNX_TEST_H_
#define YEW_LYNX_TEST_H_

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef uint32_t YewLynxSession;

typedef struct YewLynxBuffer {
  uint8_t* data;
  size_t len;
} YewLynxBuffer;

typedef struct YewLynxMountResult {
  YewLynxSession session;
  YewLynxBuffer response;
} YewLynxMountResult;

typedef struct YewLynxDestroyResult {
  uint32_t consumed;
  YewLynxBuffer response;
} YewLynxDestroyResult;

YewLynxMountResult yew_lynx_mount(uint32_t root_id);
YewLynxBuffer yew_lynx_dispatch(YewLynxSession session,
                                const uint8_t* event,
                                size_t event_len);
YewLynxBuffer yew_lynx_complete(YewLynxSession session,
                                const uint8_t* response,
                                size_t response_len);
YewLynxDestroyResult yew_lynx_destroy(YewLynxSession session);
void yew_lynx_buffer_free(YewLynxBuffer buffer);

#ifdef __cplusplus
}
#endif

#endif
