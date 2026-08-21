# Android native host adapter

This directory connects the Rust application ABI to the native renderer host
added by `patches/lynx/0002-0009`. It has one Java owner and one JNI lifecycle;
there is no Java `LynxModule` or application byte-buffer transport.

## Lifecycle

1. `LynxView.registerNativeRendererHost()` returns an opaque 64-bit host token.
2. `LynxNativeRendererHost.mount()` enters JNI.
3. JNI resolves `lynx_native_renderer_get_api` from the loaded Lynx library.
4. The selected Rust staticlib creates its framework backend and calls
   `lynx_element_bridge_native_mount`.
5. Rust copies `LynxNativeRendererApiV1`, acquires the renderer, applies the
   initial in-memory `CommandBatch`, and registers its timer.
6. Lynx invokes Rust event/timer callbacks synchronously; Rust validates IDs,
   runs the framework, and directly applies the resulting mutations.
7. `destroy()` applies framework teardown and releases the renderer. If normal
   destroy fails without consuming the token, `abandon()` is the emergency
   cleanup path.

Java clears its session whenever native returns `consumed=1`, including status
failures. Wrong-thread or busy calls return `consumed=0` and retain ownership.

## Native Contracts

- `include/lynx_native_application.h` declares session ownership, mount,
  destroy, abandon, and backend identity.
- `include/lynx_native_renderer.h` declares the versioned host function table,
  callbacks, statuses, and opaque handles.
- The patched Lynx public renderer header must byte-match the root copy.
- Event `content_type` and payload spans are borrowed for one callback and
  copied into opaque Rust bytes before framework entry.
- All lifecycle work stays on the `ALL_ON_UI` mounting thread.

`dlsym` is intentional: the standalone app links the Rust/JNI shared library
separately from Lynx's shared library. A missing export fails mount before a
Rust session is published.

## Verification

Run:

```bash
bash adapters/android/test/run-mock-checks.sh
```

The script checks the native-only Activity/Gradle wiring, Java owner semantics,
JNI status mapping and resolver failure, required JNI exports, absence of the
removed module JNI prefix, both real Rust static-library links, and required and
forbidden application C symbols.

The final 2026-08-22 patch-0009 release-safe Yew and Dioxus runs both passed on
an anonymous physical OPPO PGBM10 device running Android 13/API 33, arm64-v8a.
Each passed fresh launch, timer, tap, recreation, force-stop/reopen, and three
cycles; all six acceptance result flags were true for both. Yew recorded
`onCreate=9`, `onDestroy=4`, `diagnostics=9`, and nine selected-backend markers.
Dioxus recorded `onCreate=10`, `onDestroy=5`, `diagnostics=10`, and ten
selected-backend markers because the OS performed one extra recreation. Both
recorded `renderer_mode=native`, `bts_runtime=false`, `mts_context=false`, and
`template=false`, with zero wrong-backend, crash, or timer-teardown markers.
Evidence is at
`.deps/android/device-acceptance-native-yew-20260822-release-safe-success` and
`.deps/android/device-acceptance-native-dioxus-20260822-release-safe-success`.

The stock Lynx AAR still packages and loads Quick, PrimJS, and NAPI;
binary-native packaging remains a blocked follow-up milestone, and complete
JS-engine removal is not claimed.
