# Android LynxModule adapter

This directory contains the framework-neutral JNI adapter between Lynx's
synchronous MTS module path and one selected Rust counter backend. It uses the
public `LynxModule` surface and does not bind hidden Lynx C++ symbols.

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
builder.registerModule(
    LynxElementBridgeModule.NAME, LynxElementBridgeModule.class);
LynxView view = builder.build(context);
```

The module name is `LynxElementBridge`. One Java module instance owns at most
one live Rust session. Methods are synchronized to prevent concurrent access to
that session. `backendName()` obtains `yew` or `dioxus` from the linked Rust
archive rather than from a Java build constant. JNI validates the stable
`lynx-element-bridge-backend:<backend>` marker before returning the short name.

## MTS Surface

All payloads are FlatBuffers v2 buffers with file identifier `LEB2`:

```js
const module = lynx.module('LynxElementBridge');
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

The JNI source includes the shared `include/lynx_element_bridge.h`. Yew and
Dioxus static libraries each export the same ABI, and one is linked per build:

```c
typedef uint32_t LynxElementBridgeSession;

LynxElementBridgeMountResult lynx_element_bridge_mount(uint32_t root_id);
LynxElementBridgeBuffer lynx_element_bridge_dispatch_event(
    LynxElementBridgeSession session, const uint8_t* event, size_t event_len);
LynxElementBridgeBuffer lynx_element_bridge_complete_batch(
    LynxElementBridgeSession session,
    const uint8_t* response,
    size_t response_len);
LynxElementBridgeDestroyResult lynx_element_bridge_destroy_session(
    LynxElementBridgeSession session);
void lynx_element_bridge_buffer_free(LynxElementBridgeBuffer buffer);
const char* lynx_element_bridge_backend(void);
const char* lynx_element_bridge_backend_marker(void);
```

Input buffers are borrowed only for the call. Rust owns every returned buffer
until JNI calls `lynx_element_bridge_buffer_free` exactly once. JNI copies
responses into Java byte arrays and frees Rust allocations on both success and
Java allocation failure. No Rust panic or C++ exception may cross the ABI.

`lynx_element_bridge_destroy_session` returns `consumed=0` when a token remains
live, including a wrong-thread call. Once it returns `consumed=1`, Java clears
the token even if response copying fails. The Yew archive retains the original
`yew_lynx_*` names only as thin source-compatibility aliases.

## Build Integration

Build one Rust archive per packaged Android ABI and stage the selected backend
under `target/android-libs/<backend>/<abi>/liblynx_element_bridge_backend.a`.
`CMakeLists.txt` imports that archive and links `liblynx_element_bridge.so`.
Backend-specific Gradle and `buildStagingDirectory` paths prevent AGP/CMake
cache reuse.

`gradle-integration.gradle.kts` demonstrates the arm64 staging and CMake setup.
No Lynx Maven coordinate is prescribed because this adapter targets the pinned
source revision and uses the consuming application's Lynx build.

For the repository's full Android pipeline, run:

```bash
./scripts/build-android.sh
./scripts/build-android.sh --backend dioxus
```

That script temporarily applies the pinned Lynx ByteArray patch, builds the
required AARs and APK, and reverses the patch on exit.

## Mock Checks

Run the stock-API Java checks, JNI binary round trip, C header checks, and both
real host staticlib smoke tests with:

```bash
bash adapters/android/test/run-mock-checks.sh
```

The repository also assembles isolated arm64 APKs for both backends. On
2026-08-20, both backends independently passed the Android 15/API 35 arm64
physical-device acceptance flow. See `COMPATIBILITY.md` for the exact devices,
APK hashes, and evidence boundary.
