# Android LynxModule adapter

This directory is a minimal JNI adapter between stock OSS Lynx's synchronous
MTS module path and the Rust counter C ABI. It uses the public `LynxModule`
surface, does not vendor or patch Lynx, and does not call hidden Lynx C++
symbols. Direct JNI calls into those symbols remain unsupported.

The included checks use stock-API Java stubs and a mock Rust C ABI. They do not
link a complete Lynx SDK, build an APK, load a template, or run on a device.

## Stock Lynx API audit

The Java surface was checked against OSS Lynx commit
[`0df14207cebb060f1bed8de12b64a1119dee8f06`](https://github.com/lynx-family/lynx/tree/0df14207cebb060f1bed8de12b64a1119dee8f06):

- [`LynxModule`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/jsbridge/LynxModule.java)
  has public `(Context)` and `(Context, Object)` constructors and a public
  `void destroy()` lifecycle hook.
- [`LynxMethod`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/jsbridge/LynxMethod.java)
  is a runtime marker with no method-name override.
- [`LynxMethodWrapper`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/jsbridge/LynxMethodWrapper.java)
  supports synchronous `String` parameters and return values.
- [`LynxBaseConfigurator`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/tasm/group/LynxBaseConfigurator.java)
  publicly exposes per-runtime `registerModule(...)` and
  `setEnableMTSModule(...)`, with MTS modules disabled by default.
- [`LynxModuleFactory.AbstractLifecycleListener`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/jsbridge/LynxModuleFactory.java)
  documents teardown as running on the module execution thread, such as the JS
  thread. MTS invokes the Java method inline on the current TASM runner; that
  runner can be the Android UI thread under `ALL_ON_UI`/`PART_ON_LAYOUT`, or a
  dedicated TASM thread under other rendering strategies. The adapter does not
  marshal threads. Its methods are synchronized so accidental concurrent calls
  cannot race the opaque session, and Rust work must remain bounded because the
  entire call is synchronous.

There is one unavoidable stock Java API conflict: Java cannot implement both
the inherited `void destroy()` lifecycle method and a no-argument `String
destroy()` method because return type alone cannot overload a method. The
adapter therefore leaves the correctly overridden `void destroy()` lifecycle
hook unannotated and exposes `destroySession()` as the MTS-callable method that
returns cleanup JSON. `destroySession()` destroys only a live Rust session and,
when native reports that token consumed, permits a later `mount(String)` on the
same Java module. Inherited `destroy()` permanently marks the module destroyed,
attempts session cleanup, and rejects every later module call.

All Rust-controlled text crosses JNI as `byte[]` and is decoded in Java with
`StandardCharsets.UTF_8`. The bridge never calls `NewStringUTF`.

## Registration

Register this as a normal, per-runtime module before building the `LynxView`:

```java
LynxViewBuilder builder = new LynxViewBuilder();
builder.setEnableMTSModule(true);
builder.registerModule(YewLynxModule.NAME, YewLynxModule.class);
LynxView view = builder.build(context);
```

MTS module access is disabled by default at the audited revision, so the opt-in
above is required. The resulting MTS module name is `YewLynx`. Do not register
it as a shared module: one Java module instance owns at most one live Rust
session at a time.

The consuming host must also build one Rust archive per packaged Android ABI,
link and package `libyew_lynx_bridge.so`, and load
`../mts/dist/yew-lynx-counter.lynx.bundle` through the ordinary `LynxView`
template path. This repository does not supply that application shell.

Calls use the stock synchronous MTS broker:

```js
const module = lynx.module('YewLynx');
const mountResponseJson = module.invoke('mount', String(rootId));
const eventResponseJson = module.invoke('dispatch', String(listenerId), eventName);
const cleanupResponseJson = module.invoke('destroySession');
```

The MTS side remains responsible for parsing each JSON response and applying
its public Element API operations. Java treats native JSON as opaque UTF-8
text, while every Java-generated fallback uses the same exact protocol-v1
success/failure envelope. `lynx.module()` is implemented by stock MTS at the
audited revision, but it is not declared by that revision's stable public
TypeScript `Lynx` interface; this is therefore a pinned integration surface,
not a stable TypeScript API commitment.

## Exact Rust C ABI

The JNI bridge includes the current Rust-owned
`examples/counter/include/yew_lynx.h` directly. The `staticlib` linked into the
bridge must export this compatibility C ABI with unmangled symbol names:

```c
#include <stddef.h>
#include <stdint.h>

#define YEW_LYNX_JS_MAX_SAFE_INTEGER UINT64_C(9007199254740991)

typedef uint64_t YewLynxSession;

typedef struct YewLynxBuffer {
  uint8_t *data;
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

YewLynxMountResult yew_lynx_mount(const uint8_t *root_id,
                                  size_t root_id_len);
YewLynxBuffer yew_lynx_dispatch(YewLynxSession session,
                                const uint8_t *listener_id,
                                size_t listener_id_len,
                                const uint8_t *event_name,
                                size_t event_name_len);
YewLynxDestroyResult yew_lynx_destroy(YewLynxSession session);
void yew_lynx_buffer_free(YewLynxBuffer buffer);
```

ABI requirements:

- Root and listener IDs are positive, JavaScript-safe unsigned decimal UTF-8
  byte spans. Event names are exact UTF-8 byte spans. Rust borrows all input
  pointers only for the call.
- Every returned response is valid UTF-8 JSON with exactly one of the protocol
  envelopes documented in `../mts/PROTOCOL.md`. Rust owns its allocation until the
  bridge calls `yew_lynx_buffer_free` exactly once. `{NULL, 0}` is a valid empty
  buffer; `{NULL, nonzero}` is invalid.
- `yew_lynx_mount` returns an integer capability token and initial response. On
  failure it returns token zero and a JSON failure response.
- `yew_lynx_dispatch` consumes UTF-8 listener/event bytes with a live token.
- `yew_lynx_destroy` reports `consumed=0` when the token remains live, such as a
  wrong-thread call, and `consumed=1` once the caller must clear it, including a
  teardown failure that consumed the session. Its response may then carry the
  valid partial cleanup sequence allowed by the destroy protocol.
- All calls for a session must run on its mounting thread.
- The Rust exports must catch panics; no panic or C++ exception may cross the C
  ABI. `panic = "abort"` cannot be caught or cleaned up. Calls for one session
  are serialized by Java.

The JNI bridge copies every returned response into a Java byte array and frees
the Rust buffer on success and JNI allocation failure. It also destroys a newly
mounted session if Java cannot receive its token. JNI writes native `consumed`
to Java before copying the destroy response, and Java clears its token only when
that bit is set. This preserves a live token after retryable failures while
still clearing a consumed token if response copying or decoding fails.

## Android build integration

Build the Rust crate as a `staticlib` named `libyew_lynx_counter.a`, with one file
per enabled Android ABI:

```text
target/android-libs/
  arm64-v8a/libyew_lynx_counter.a
  armeabi-v7a/libyew_lynx_counter.a
  x86_64/libyew_lynx_counter.a
```

`CMakeLists.txt` imports that public C archive and links it into
`libyew_lynx_bridge.so`. Java loads the bridge, so the consuming APK packages
only the resulting shared library. The CMake target also links the Android
system libraries reported by Rust 1.85 for `aarch64-linux-android`; a Rust or
target change requires rerunning `cargo rustc -- --print native-static-libs`
and preserving the reported order.

`gradle-integration.gradle.kts` contains Kotlin DSL blocks to add the Java
source and CMake build to an Android module that already has its Lynx dependency.
The supplied block currently enables only `arm64-v8a`, builds the
`aarch64-linux-android` archive, and stages it into the exact directory consumed
by CMake before `preBuild`/CMake configuration. Add equivalent target-to-ABI
tasks before enabling another ABI.

No Lynx Maven coordinate is prescribed here because this adapter targets the
audited source revision and must use the consuming application's existing public
Lynx build.

The host must explicitly call `destroySession()` for a removable/reloadable
session and route final runtime teardown through inherited `destroy()`. A
consumed `destroySession()` permits a later mount on the same live module;
module `destroy()` is permanent. Neither dropping a Rust handle nor owner-thread
exit performs host cleanup.

## Mock checks

The adapter includes stock-API Java stubs plus a mock Rust C ABI that is linked
through the real JNI source. It also links a C smoke program to the real host
Rust `staticlib`. Run the Java lifecycle/schema checks, JNI round-trip,
evolved-header build, repository-header syntax build, and real C ABI smoke test
with:

```sh
bash test/run-mock-checks.sh
```

Repository verification separately builds the Rust archive for
`aarch64-linux-android`. It does not provide an NDK or perform the final Android
shared-library/APK link.

Passing these checks is source/API/mock evidence only, not stock Lynx runtime or
device support.
