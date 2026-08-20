#ifndef YEW_LYNX_H_
#define YEW_LYNX_H_

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define YEW_LYNX_ABI_VERSION 2u
#define YEW_LYNX_FIBER_PROTOCOL_VERSION 2u

typedef uint32_t yew_lynx_counter_status_t;

#define YEW_LYNX_COUNTER_STATUS_OK ((yew_lynx_counter_status_t)0u)
#define YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT ((yew_lynx_counter_status_t)1u)
#define YEW_LYNX_COUNTER_STATUS_INVALID_SESSION ((yew_lynx_counter_status_t)2u)
#define YEW_LYNX_COUNTER_STATUS_WRONG_THREAD ((yew_lynx_counter_status_t)3u)
#define YEW_LYNX_COUNTER_STATUS_UNSUPPORTED ((yew_lynx_counter_status_t)4u)
#define YEW_LYNX_COUNTER_STATUS_INVALID_OWNERSHIP ((yew_lynx_counter_status_t)5u)
#define YEW_LYNX_COUNTER_STATUS_INVALID_LISTENER ((yew_lynx_counter_status_t)6u)
#define YEW_LYNX_COUNTER_STATUS_RESOURCE_EXHAUSTED ((yew_lynx_counter_status_t)7u)
#define YEW_LYNX_COUNTER_STATUS_HOST_ERROR ((yew_lynx_counter_status_t)8u)
#define YEW_LYNX_COUNTER_STATUS_PANIC ((yew_lynx_counter_status_t)9u)
#define YEW_LYNX_COUNTER_STATUS_INTERNAL_ERROR ((yew_lynx_counter_status_t)10u)

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
  /* One only when the owner-thread call consumed the supplied session token. */
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

/*
 * IDs are nonzero opaque 32-bit integers scoped to one session. Event and
 * completion inputs, and every returned response, are FlatBuffers v2 `LEB2`
 * envelopes. Returned buffers must be freed exactly once.
 *
 * Successful calls return a CommandBatch on the Command channel with a final
 * commit boundary. Failed calls return a ResponseBatch on the Result channel.
 * No protocol v1 JSON or decimal-string ID encoding is accepted.
 *
 * All session calls must stay on the mounting thread. A wrong-thread destroy
 * returns consumed=0 so the owner can retry. Once consumed=1, the caller must
 * clear its token even if the response is a failure. Session tokens are integer
 * capabilities and are never pointers.
 *
 * Thread exit releases Rust's global owner bookkeeping and abandons its local
 * application state, but cannot clean host ElementRefs or callbacks. Hosts must
 * explicitly call yew_lynx_destroy on the owner thread before that thread or
 * its page exits.
 */

#ifdef __cplusplus
}  /* extern "C" */
#endif

#endif  /* YEW_LYNX_H_ */
