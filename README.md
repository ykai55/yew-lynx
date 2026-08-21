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
- Yew patch base: `0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3`
- Dioxus Core: `0.7.10`
- Rust: `1.85.0`

`third_party/lynx` is the audited upstream gitlink. `patches/lynx/0002-0009`
add the native renderer API, Android host registry, lifecycle validation,
diagnostics, event delivery, and focused boundary tests. `patches/yew` adds the
host-independent native renderer used by the Yew adapter.

## Architecture

- `crates/element-bridge-core/` owns nonzero IDs, owner-thread checks, tree and
  listener validation, ordered `CommandBatch` mutations, opaque event payloads,
  deterministic teardown, and `HostFake`.
- `adapters/yew/` maps patched Yew renderer calls into the core session.
- `adapters/dioxus/` maps Dioxus 0.7.10 `WriteMutations` calls while retaining
  Dioxus framework `Template` types inside that adapter.
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

Final patch-0009 release-safe acceptance passed on 2026-08-22 for both backends
on an anonymous physical OPPO PGBM10 device running Android 13/API 33,
arm64-v8a:

| Backend | APK SHA-256 | Evidence | `onCreate` | `onDestroy` | diagnostics/backend |
| --- | --- | --- | ---: | ---: | ---: |
| Yew | `685e8000ac037607fc9cd870d0445293c3f11ec11d6c00b2a375033ed468a1bf` | `.deps/android/device-acceptance-native-yew-20260822-release-safe-success` | 9 | 4 | 9 |
| Dioxus | `6a51be912a01764566edbf8bea859effe0af073116a785fd6251d71f53664513` | `.deps/android/device-acceptance-native-dioxus-20260822-release-safe-success` | 10 | 5 | 10 |

Both runs passed fresh launch, timer, tap, recreation, force-stop/reopen, and
three repeated cycles, with all six acceptance result flags true. Both recorded
`renderer_mode=native`, `bts_runtime=false`, `mts_context=false`, and
`template=false`, with zero wrong-backend, crash, or timer-teardown markers.
Dioxus has one additional create, destroy, diagnostic, and selected-backend
marker because the OS performed one extra recreation.

The stock Lynx AAR still packages and loads Quick, PrimJS, and NAPI;
binary-native packaging remains a blocked follow-up milestone, and complete
JS-engine removal is not claimed.

## Build And Verify

Prepare the pinned Yew checkout and run the complete host-independent suite:

```bash
./scripts/bootstrap-yew.sh
./scripts/verify.sh
```

Verification covers Rust formatting/check/test/Clippy, Yew/Dioxus conformance,
public native lifecycle tests, Android Java/JNI mocks, both real static-library
links, required/forbidden exported symbols, sequential Lynx patch application,
public-header identity, and patched-Yew tests.

Build an Android APK only when needed:

```bash
./scripts/build-android.sh --backend yew
./scripts/build-android.sh --backend dioxus
```

The Android build requires JDK 11, the documented Android SDK/NDK versions,
Rust 1.85.0, and Node.js only for Lynx's own source build tooling. The
application rendering path is native-only and the build rejects any packaged
`.lynx.bundle`.

Do not infer compatibility across pin changes. See
[`COMPATIBILITY.md`](COMPATIBILITY.md) and
[`docs/adapter-authoring.md`](docs/adapter-authoring.md).

## Licensing

This repository is Apache-2.0. Upstream projects retain their own terms; see
[`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
