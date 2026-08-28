# Lynx native renderer patch series

This directory contains the native renderer changes applied to the pinned
public Lynx source. The maintained series is the 15 patches `0002-0016`.

## Base Revision

- Upstream: <https://github.com/lynx-family/lynx>
- Commit: `0df14207cebb060f1bed8de12b64a1119dee8f06`

The patches add, in order:

- A versioned C function table with opaque renderer, node, listener, callback,
  and timer handles.
- Android `LynxView` host registration for `ALL_ON_UI` and host invalidation
  before renderer teardown.
- Native page/Fiber configuration and deterministic lifecycle cleanup.
- Handle generation, rollback, callback containment, and rebuild hardening.
- Runtime diagnostics and production function-table boundary tests.
- Platform event subscription and exact listener lifecycle.
- Template-ready event dispatch state for native sessions, restored on teardown.
- Acquire-time release preparation so valid owner-thread release is infallible.
- An opt-in Android native-renderer-only build without QuickJS, NAPI, Wasm, V8,
  or JavaScript runtime integration.
- A separate native renderer library/product with explicit rejection for
  unsupported runtime, template, and engine-cache entry points.
- Native-only environment initialization that skips the intentionally absent
  template/runtime cache cleanup JNI path.
- Native-only Android platform event plumbing through `LynxEventEmitter` and
  the renderer-owned engine proxy, without restoring the layout or JS runtime
  initialization paths.
- Production function-table, callback, and task-runner coverage for unsupported
  callbacks, panic containment, and all public statuses through real production
  behavior without private test hooks.
- A complete native-only Java/JNI boundary: supported renderer and shell methods
  remain linked, while SSR, runtime, template, and engine-reuse APIs reject in
  Java before intentionally omitted JNI methods can be reached.
- A publicly reachable tools_shared pin in Habitat's primary `dependencies/DEPS`
  file for reproducible clean-checkout builds; the unavailable child revision
  differed only in an iOS packaging helper.

Apply patches strictly in `series` order. Other Lynx revisions require a rebase
and complete reverification. Native-only Android builds also apply the separate
`patches/lynx-tools-shared` series to the Lynx-pinned tools_shared checkout at
`ff47fee7d41ee3e8e8561041b1ce2c8b50e923ea` for reproducible JNI registration
filtering.

## Android Products

The default build remains the stock
`org.lynxsdk.lynx:lynx:0.0.1-0df14207` / `liblynx.so` product with unchanged
JavaScript/template behavior. The opt-in native-only build publishes
`org.lynxsdk.lynx:lynx-native-renderer:0.0.1-0df14207` /
`liblynx_native_renderer.so` separately. Its POM excludes stock `lynx`,
Quick/PrimJS, NAPI, Wasm, V8, and LynxJSSDK dependencies. Its ELF exports
`lynx_native_renderer_get_api` without forbidden runtime `DT_NEEDED` entries or
undefined runtime symbols.

The native renderer continues to expose the versioned C function table; the
separate product does not introduce MTS or template transport.

## Verification

`scripts/verify.sh` applies every patch sequentially to the clean pinned
submodule, byte-compares
`third_party/lynx/core/public/lynx_native_renderer.h` with
`include/lynx_native_renderer.h`, and reverses the applied patches in reverse
order even on failure.

All 21 `NativeRendererApiTest` cases pass (21/21). There is no private test peer
or helper; release behavior is tested through the production function table.
The cases cover function-table version/size validation, null and malformed
spans, UTF-8, wrong-thread, stale and foreign handles, acquire/release rollback,
listener identity, event delivery, native-only EventEmitter/engine-proxy setup,
platform tap registration, timers, lifecycle restoration, non-consuming
wrong-thread release, owner release and reacquisition, missing callbacks,
callback failures, and host exceptions. All 11 public status values cross the
production function-table boundary as API results or callback results with
observable production behavior:

| Status | Production function-table coverage |
| --- | --- |
| `OK` | Positive acquire, tree, listener, timer, flush, and release paths. |
| `INVALID_ARGUMENT` | Null outputs/callback table, malformed spans, invalid UTF-8. |
| `INVALID_SESSION` | Stale renderer/host calls and repeated release. |
| `WRONG_THREAD` | Cross-thread mutation, root lookup, and release. |
| `UNSUPPORTED` | Missing event and timer callbacks. |
| `INVALID_OWNERSHIP` | Foreign/stale nodes and timers plus invalid tree mutations. |
| `INVALID_LISTENER` | Listener identity mismatch. |
| `RESOURCE_EXHAUSTED` | A real repeating timer callback returns the status through the function table, and production task-runner behavior cancels the timer after one callback. |
| `HOST_ERROR` | A pre-existing root rejects acquire without consuming the host registration. |
| `PANIC` | Exception-enabled acquire catches a host task-runner exception, rolls back, and returns `PANIC`; callback-returned `PANIC` also cancels repeating timers. |
| `INTERNAL_ERROR` | A real repeating timer callback returns the status through the function table, and production task-runner behavior cancels the timer after one callback. |

Neither `RESOURCE_EXHAUSTED` nor `INTERNAL_ERROR` uses allocator exhaustion or
root rollback. This production-boundary coverage and the final patch-0015 device
evidence complete Issue #4, so it is closable.

The Java instrumentation coverage also calls the supported long-task and timing
JNI paths, verifies the native-only `ALL_ON_UI` event-thread result without JNI,
and requires unsupported SSR/runtime methods to throw before JNI dispatch.

Compile the Android changes in a patched Lynx checkout with:

```bash
cd platform/android
./gradlew :LynxAndroid:compileNoasanReleaseJavaWithJavac \
  :LynxAndroid:compileNoasanDebugAndroidTestJavaWithJavac \
  -x :LynxAndroid:extractJNIFiles
```

From the patched Lynx root, build the default and native-renderer-only shared
libraries with:

```bash
source tools/envsetup.sh
platform/android/gradlew -p platform/android \
  :LynxAndroid:externalNativeBuildNoasanRelease \
  -PabiList=arm64-v8a -PbuildLynxDebugSo \
  -x :LynxAndroid:extractJNIFiles --no-daemon
platform/android/gradlew -p platform/android \
  :LynxAndroid:externalNativeBuildNoasanRelease \
  -Penable_native_renderer_only=true \
  -PabiList=arm64-v8a -PbuildLynxDebugSo \
  -x :LynxAndroid:extractJNIFiles --no-daemon
```
