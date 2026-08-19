#ifndef YEW_LYNX_H_
#define YEW_LYNX_H_

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define YEW_LYNX_ABI_VERSION 1u
#define YEW_LYNX_FIBER_PROTOCOL_VERSION 1u
#define YEW_LYNX_JS_MAX_SAFE_INTEGER UINT64_C(9007199254740991)

typedef uint32_t yew_lynx_counter_status_t;

#define YEW_LYNX_COUNTER_STATUS_OK ((yew_lynx_counter_status_t)0u)
#define YEW_LYNX_COUNTER_STATUS_INVALID_ARGUMENT ((yew_lynx_counter_status_t)1u)
#define YEW_LYNX_COUNTER_STATUS_INVALID_UTF8 ((yew_lynx_counter_status_t)2u)
#define YEW_LYNX_COUNTER_STATUS_INVALID_SESSION ((yew_lynx_counter_status_t)3u)
#define YEW_LYNX_COUNTER_STATUS_WRONG_THREAD ((yew_lynx_counter_status_t)4u)
/* Reserved in ABI v1; roots are scoped to sessions and this status is not emitted. */
#define YEW_LYNX_COUNTER_STATUS_DUPLICATE_ROOT ((yew_lynx_counter_status_t)5u)
#define YEW_LYNX_COUNTER_STATUS_INVALID_LISTENER ((yew_lynx_counter_status_t)6u)
#define YEW_LYNX_COUNTER_STATUS_EVENT_MISMATCH ((yew_lynx_counter_status_t)7u)
#define YEW_LYNX_COUNTER_STATUS_BACKEND_ERROR ((yew_lynx_counter_status_t)8u)
#define YEW_LYNX_COUNTER_STATUS_PANIC ((yew_lynx_counter_status_t)9u)
#define YEW_LYNX_COUNTER_STATUS_SESSION_POISONED ((yew_lynx_counter_status_t)10u)
#define YEW_LYNX_COUNTER_STATUS_RESOURCE_EXHAUSTED ((yew_lynx_counter_status_t)11u)
#define YEW_LYNX_COUNTER_STATUS_INTERNAL_ERROR ((yew_lynx_counter_status_t)12u)

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
  /* One only when the owner-thread call consumed the supplied session token. */
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

/*
 * Inputs are exact, non-NUL-terminated UTF-8 spans. Root, node, listener, and
 * session IDs are positive integers no greater than Number.MAX_SAFE_INTEGER.
 * Every returned response buffer is UTF-8 JSON and must be freed exactly once.
 *
 * Every successful wire call has exactly these fields:
 *   {"version":1,"ok":true,"operations":[...]}
 * Every failed wire call has exactly these fields:
 *   {"version":1,"ok":false,"status":N,"error":"...","operations":[...]}
 * A successful operations array contains exactly one final flush. A failed
 * call normally has no operations and never exposes mutations from a poisoned
 * backend or session.
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
