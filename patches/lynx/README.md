# Lynx native renderer patch series

This directory contains the native renderer changes applied to the pinned
public Lynx source. The maintained series is `0002-0009`.

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

Apply patches strictly in `series` order. Other Lynx revisions require a rebase
and complete reverification.

## Verification

`scripts/verify.sh` applies every patch sequentially to the clean pinned
submodule, byte-compares
`third_party/lynx/core/public/lynx_native_renderer.h` with
`include/lynx_native_renderer.h`, and reverses the applied patches in reverse
order even on failure.

The patch tests cover function-table version/size validation, null and malformed
spans, UTF-8, wrong-thread, stale and foreign handles, acquire/release rollback,
listener identity, event delivery, timers, lifecycle restoration, release-option
prepublication, and setup-failure root/host rollback.

Compile the Android changes in a patched Lynx checkout with:

```bash
cd platform/android
./gradlew :LynxAndroid:compileNoasanReleaseJavaWithJavac \
  :LynxAndroid:compileNoasanDebugAndroidTestJavaWithJavac \
  -x :LynxAndroid:extractJNIFiles
```
