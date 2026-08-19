#ifndef YEW_LYNX_TEST_H_
#define YEW_LYNX_TEST_H_

#include <stddef.h>
#include <stdint.h>

#define YEW_LYNX_JS_MAX_SAFE_INTEGER UINT64_C(9007199254740991)

#ifdef __cplusplus
extern "C" {
#endif

typedef uint64_t YewLynxSession;

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

YewLynxMountResult yew_lynx_mount(const uint8_t* root_id,
                                  size_t root_id_len);
YewLynxBuffer yew_lynx_dispatch(YewLynxSession session,
                                const uint8_t* listener_id,
                                size_t listener_id_len,
                                const uint8_t* event_name,
                                size_t event_name_len);
YewLynxDestroyResult yew_lynx_destroy(YewLynxSession session);
void yew_lynx_buffer_free(YewLynxBuffer buffer);

#ifdef __cplusplus
}
#endif

#endif
