# Android LynxModule adapter

This directory contains the JNI adapter between Lynx's synchronous MTS module
path and the Rust counter C ABI. It uses the public `LynxModule` surface and does
not bind hidden Lynx C++ symbols.

The adapter targets Lynx commit
`0df14207cebb060f1bed8de12b64a1119dee8f06`. At that revision:

- `LynxModule` provides public constructors and a `void destroy()` lifecycle
  hook.
- `LynxMethodWrapper` supports synchronous numeric and `byte[]` parameters and
  return values.
- `LynxBaseConfigurator` exposes per-runtime module registration and the MTS
  opt-in, which is disabled by default.
- Module teardown runs on the module execution thread. The adapter does not
  marshal threads, and Rust requires every session call to remain on its mount
  thread.

The pinned Lynx patch under `patches/lynx` exposes Java ByteArray values to
ordinary LepusNG as a read-only byte view. It is required for MTS to read the
returned FlatBuffers buffers directly.

## Registration

Register one non-shared module per runtime before building the `LynxView`:

```java
LynxViewBuilder builder = new LynxViewBuilder();
builder.setEnableMTSModule(true);
builder.registerModule(YewLynxModule.NAME, YewLynxModule.class);
LynxView view = builder.build(context);
```

The module name is `YewLynx`. One Java module instance owns at most one live
Rust session. Methods are synchronized to prevent concurrent access to that
session.

## MTS Surface

All payloads are FlatBuffers v2 buffers with file identifier `LEB2`:

```js
const module = lynx.module('YewLynx');
const mountCommands = module.invoke('mount', rootId);
const eventCommands = module.invoke('dispatchEvent', eventBytes);
const completionResult = module.invoke('completeBatch', resultBytes);
const cleanupCommands = module.invoke('destroySession');
```

IDs cross Java as positive unsigned 32-bit values represented by `long` because
Java has no unsigned `int`. Event and completion envelopes cross unchanged as
`byte[]`. Java-generated failures are valid Result-channel `LEB2` envelopes.

Java cannot expose an MTS method named `destroy` while overriding inherited
`void destroy()`, so `destroySession()` is the callable teardown method. A
consumed `destroySession()` permits a later mount on the same live module;
inherited `destroy()` permanently closes the module.

## Rust C ABI

The JNI source includes `examples/counter/include/yew_lynx.h`. The linked
`libyew_lynx_counter.a` exports:

```c
typedef uint32_t YewLynxSession;

YewLynxMountResult yew_lynx_mount(uint32_t root_id);
YewLynxBuffer yew_lynx_dispatch(YewLynxSession session,
                                const uint8_t* event,
                                size_t event_len);
YewLynxBuffer yew_lynx_complete(YewLynxSession session,
                                const uint8_t* response,
                                size_t response_len);
YewLynxDestroyResult yew_lynx_destroy(YewLynxSession session);
void yew_lynx_buffer_free(YewLynxBuffer buffer);
```

Input buffers are borrowed only for the call. Rust owns every returned buffer
until JNI calls `yew_lynx_buffer_free` exactly once. JNI copies responses into
Java byte arrays and frees Rust allocations on both success and Java allocation
failure. No Rust panic or C++ exception may cross the ABI.

`yew_lynx_destroy` returns `consumed=0` when a token remains live, including a
wrong-thread call. Once it returns `consumed=1`, Java clears the token even if
response copying fails.

## Build Integration

Build one Rust archive per packaged Android ABI and stage it under
`target/android-libs/<abi>/libyew_lynx_counter.a`. `CMakeLists.txt` imports the
archive and links `libyew_lynx_bridge.so`; the consuming APK packages the shared
library.

`gradle-integration.gradle.kts` demonstrates the arm64 staging and CMake setup.
No Lynx Maven coordinate is prescribed because this adapter targets the pinned
source revision and uses the consuming application's Lynx build.

For the repository's full Android pipeline, run:

```bash
./scripts/build-android.sh
```

That script temporarily applies the pinned Lynx ByteArray patch, builds the
required AARs and APK, and reverses the patch on exit.

## Mock Checks

Run the stock-API Java checks, JNI binary round trip, C header checks, and real
host staticlib smoke test with:

```bash
bash adapters/android/test/run-mock-checks.sh
```

These checks do not load a complete Lynx runtime or prove device support. See
`COMPATIBILITY.md` for the current evidence boundary.
