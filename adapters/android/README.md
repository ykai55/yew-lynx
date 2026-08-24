# Android native host adapter

This directory connects the Rust application ABI to the native renderer host
added by the 15-patch `patches/lynx` series (`0002-0016`). It has one Java owner
and one JNI lifecycle; there is no Java `LynxModule` or application byte-buffer
transport.

The app opts into
`org.lynxsdk.lynx:lynx-native-renderer:0.0.1-0df14207`, which packages
`liblynx_native_renderer.so`. The separately built and published stock
`org.lynxsdk.lynx:lynx:0.0.1-0df14207` product packages `liblynx.so` and retains
its existing JavaScript/template behavior; it is not an app dependency. The
runtime path remains the versioned C ABI and does not use MTS or template
transport.

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

The Android build additionally verifies that the native product and app graphs
exclude stock `lynx`, Quick/PrimJS, NAPI, Wasm, V8, and LynxJSSDK; that the APK
excludes their shared libraries and `assets/lynx_core.js`; and that
`liblynx_native_renderer.so` has no forbidden runtime `DT_NEEDED` entries or
undefined runtime symbols and exports `lynx_native_renderer_get_api`.

The final 2026-08-22 patch-0015 binary-native Yew and Dioxus runs both passed on
an anonymous Xiaomi Redmi K60 Pro physical device running Android 13/API 33,
arm64-v8a.
Each passed fresh launch, timer, tap, recreation, force-stop/reopen, and three
cycles; every functional result flag was true for both. Each recorded
`onCreate=9`, `onDestroy=4`, `diagnostics=9`, nine selected-backend markers,
`renderer_mode=native`, `bts_runtime=false`, `mts_context=false`, and
`template=false`, with zero crash markers. `proc_maps_checked` is true: all five
required libraries were mapped after fresh launch and interaction, the
forbidden set was empty, and Quick/NAPI/Wasm/V8 mapping flags were false.
The Yew APK SHA-256 is
`121c20a6bc82d1570eb24cfb37c84a5d82c414bc1b176c4b40ac8294fe45903d`, with
evidence at
`.deps/android/device-acceptance-binary-native-yew-20260822-0015-final`. The
Dioxus APK SHA-256 is
`008d64415874bff997a47c1afcb25dc2e87c5fe4e6f513acaad2bf8273f3afdc`, with
evidence at
`.deps/android/device-acceptance-binary-native-dioxus-20260822-0015-final`.
