# Lynx Element Bridge

> [!WARNING]
> **Experimental public preview.** This is an independent research project,
> not an officially supported Lynx, Yew, or Dioxus integration. Compatibility
> is limited to the pinned revisions and verification described here.

Lynx Element Bridge mounts Rust UI frameworks directly into Lynx's Fiber DOM
through a versioned native C function table. Yew and Dioxus produce the same
in-memory Rust mutations; no template bundle, Java `LynxModule`, or serialized
command transport participates in the application lifecycle.

```text
Yew NativeRendererBackend       Dioxus WriteMutations
             \                    /
              Session -> CommandBatch
                        |
                 Rust NativeHost
                        |
          LynxNativeRendererApiV1 C table
                        |
                  Lynx Fiber DOM
```

## Pinned Inputs

- Lynx: `0df14207cebb060f1bed8de12b64a1119dee8f06`
- Lynx tools_shared: `ff47fee7d41ee3e8e8561041b1ce2c8b50e923ea`
- Yew patch base: `0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3`
- Dioxus: `0.7.10`
- Rust: `1.85.0`

`third_party/lynx` is the audited upstream gitlink. The 15-patch
`patches/lynx` series (`0002-0016`) adds the native renderer API, Android host
registry, lifecycle validation, diagnostics, event delivery, native-only Android
product, and focused boundary tests. The pinned `patches/lynx-tools-shared`
series makes native-only JNI registration filtering reproducible. `patches/yew`
adds the host-independent native renderer used by the Yew adapter.

## Architecture

- `crates/element-bridge-core/` owns nonzero IDs, owner-thread checks, tree and
  listener validation, ordered `CommandBatch` mutations, opaque event payloads,
  deterministic teardown, and `HostFake`.
- `adapters/yew/` maps patched Yew renderer calls into the core session.
- `adapters/dioxus/` provides Lynx-native `view`/`text` RSX vocabulary and maps
  Dioxus 0.7.10 `WriteMutations` calls into the core session.
- `examples/dioxus-counter/` authors the component with `rsx!`; its in-memory
  Dioxus `Template` is compile-time VDOM data, not a Lynx runtime template.
- `crates/adapter-conformance/` compares mount, event, update, and destroy
  behavior across both frameworks.
- `crates/element-bridge-ffi/` owns native sessions and contains panic,
  reentry/busy, owner-thread, poison, destroy, and abandon boundaries.
- `NativeHost` maps bridge IDs to opaque Lynx handles and applies each command
  directly through a copied `LynxNativeRendererApiV1` table.
- `include/lynx_native_renderer.h` is the host function-table contract mirrored
  by the patched Lynx public header.
- `include/lynx_native_application.h` is the self-contained application ABI:
  native mount, destroy, abandon, and backend identity.
- `adapters/android/` resolves the Lynx function table with `dlsym` and exposes
  the native lifecycle to `LynxNativeRendererHost`.

`CommandBatch` remains an in-memory Rust boundary. Its ordered `Vec<Command>`
contains mutations only. Event payload bytes and content type are copied at the
native callback boundary and remain opaque to the bridge.

## Android Products

The pinned source publishes two separate Maven products at version
`0.0.1-0df14207`; this app opts into the native product:

- `org.lynxsdk.lynx:lynx` contains the stock `liblynx.so`. It is built and
  published separately, and its JavaScript/template behavior is unchanged.
- `org.lynxsdk.lynx:lynx-native-renderer` contains
  `liblynx_native_renderer.so`. It is the product selected by this app and does
  not depend on the stock `lynx` artifact.

The native product POM and application runtime graph exclude stock `lynx`,
Quick/PrimJS, NAPI, V8, LynxJSSDK, and standalone Wasm runtime artifacts. The
APK likewise excludes `liblynx.so`, their runtime shared libraries, and
`assets/lynx_core.js`. Each APK contains a build-variant-selected Yew or Dioxus
Native runtime in `liblynx_element_bridge_native.so` and a framework-neutral
WAMR host in `liblynx_element_bridge_wamr.so`. Guests are compiled externally,
loaded only from a user-provided URL, and are never packaged in the APK. The native renderer ELF has
no forbidden runtime `DT_NEEDED` entries or undefined runtime symbols, and
exports `lynx_native_renderer_get_api`.
Packaging does not change the runtime architecture: framework mutations still
cross the versioned C function table in memory, with no MTS or template
transport.

## Public Native ABI

Each framework static library exports only:

```c
LynxElementBridgeNativeMountResult lynx_element_bridge_native_mount(
    LynxNativeRendererGetApiFn get_api,
    LynxNativeHostHandle host);
LynxElementBridgeNativeDestroyResult
lynx_element_bridge_native_destroy_session(LynxElementBridgeSession session);
LynxElementBridgeNativeDestroyResult
lynx_element_bridge_native_abandon_session(LynxElementBridgeSession session);
const char* lynx_element_bridge_backend(void);
const char* lynx_element_bridge_backend_marker(void);
```

Sessions and host handles are opaque and nonzero. Calls and callbacks are
synchronous on the mounting thread. Normal destroy applies framework teardown
before releasing the renderer. Abandon is an emergency path that consumes Rust
state without applying teardown mutations. A consumed failure still invalidates
the session token.

## Device Evidence

Final patch-0015 binary-native acceptance passed on 2026-08-22 for both backends
on an anonymous Xiaomi Redmi K60 Pro physical device running Android 13/API 33,
arm64-v8a:

| Backend | APK SHA-256 | Evidence | `onCreate` | `onDestroy` | diagnostics/backend |
| --- | --- | --- | ---: | ---: | ---: |
| Yew | `121c20a6bc82d1570eb24cfb37c84a5d82c414bc1b176c4b40ac8294fe45903d` | `.deps/android/device-acceptance-binary-native-yew-20260822-0015-final` | 9 | 4 | 9 |
| Dioxus | `008d64415874bff997a47c1afcb25dc2e87c5fe4e6f513acaad2bf8273f3afdc` | `.deps/android/device-acceptance-binary-native-dioxus-20260822-0015-final` | 9 | 4 | 9 |

Both runs passed fresh launch, tap, recreation, force-stop/reopen, and
three repeated cycles, with every functional flag true and zero crash markers.
Each recorded `onCreate=9`, `onDestroy=4`, nine native diagnostics, and nine
selected-backend markers,
`renderer_mode=native`, `bts_runtime=false`, `mts_context=false`, and
`template=false` and `proc_maps_checked=true`. Maps were captured after fresh
launch and after interaction: all five required libraries (`liblynx_native_renderer.so`,
`liblynx_element_bridge.so`, `liblynxbase.so`, `liblynxgfx.so`, and
`liblynxtrace.so`) were mapped at both points; the forbidden set was empty and
Quick, NAPI, Wasm, and V8 mapping flags were false.

The current acceptance flow writes eight evidence files. All 21 `NativeRendererApiTest`
cases pass (21/21), with no private test peer or helper; release behavior is
tested through the production function table. All 11 public statuses cross the
production boundary. `RESOURCE_EXHAUSTED` and `INTERNAL_ERROR` are returned by
real repeating timer callbacks through the function table, and production
task-runner behavior cancels each timer after one callback. Neither status uses
allocator exhaustion or root rollback. The patch-0015 tests and final device
evidence complete Issue #4's implementation and acceptance requirements, so
the issue is closable. See `patches/lynx/README.md` for the status-by-status
evidence.

## Build And Verify

Prepare the pinned Yew checkout and run the complete host-independent suite:

```bash
./scripts/bootstrap-yew.sh
./scripts/verify.sh
```

Verification covers Rust formatting/check/test/Clippy, Yew/Dioxus conformance,
public native lifecycle tests, Android Java/JNI mocks, both real static-library
links, required/forbidden exported symbols, sequential Lynx patch application,
the pinned tools_shared JNI-filter patch, public-header identity, stock/native
product separation, dependency/APK/ELF inspection, and patched-Yew tests.

Build an Android APK only when needed:

```bash
./scripts/build-android.sh --backend yew
./scripts/build-android.sh --backend dioxus
```

The Android build requires JDK 11, the documented Android SDK/NDK versions,
Rust 1.85.0, and Node.js only for Lynx's own source build tooling. PrimJS lock
and preparation inputs are used only to reproducibly build and test the
preserved stock artifact; they are not dependencies of the native product or
app. The application rendering path is native-only and the build rejects any
packaged `.lynx.bundle`.

Do not infer compatibility across pin changes. See
[`COMPATIBILITY.md`](COMPATIBILITY.md) and
[`docs/adapter-authoring.md`](docs/adapter-authoring.md).

## Licensing

This repository is Apache-2.0. Upstream projects retain their own terms; see
[`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
