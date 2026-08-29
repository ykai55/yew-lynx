# Lynx Element Bridge

English | [简体中文](README.zh-CN.md)

> [!WARNING]
> **Experimental public preview.** This is an independent research project,
> not an officially supported Lynx, Yew, or Dioxus integration. Compatibility
> is limited to the pinned revisions and verified targets described below.

Lynx Element Bridge mounts Rust UI frameworks directly into Lynx's Fiber DOM.
Yew and Dioxus render into a shared Rust mutation model, which is applied through
a versioned native C function table. The application lifecycle does not use a
Lynx template bundle, Java `LynxModule`, JavaScript/MTS, or a serialized native
command channel.

The repository currently provides:

- Yew and Dioxus adapters with equivalent observable behavior.
- Native Android runtimes linked into the APK.
- A WAMR runtime that loads external `wasm32-wasip1` applications.
- A patched, native-renderer-only Lynx Android product.
- Host, ABI, integration, artifact, and physical-device verification.

## Architecture

### Layered Design

```text
Framework layer
  Yew NativeRendererBackend          Dioxus WriteMutations
                \                    /
                 v                  v
Bridge core
  Session -> ordered CommandBatch -> tree/listener validation
                              |
Execution backend             |
  Native Rust ----------------+---------------- WAMR-hosted Rust
                              |
Native boundary               v
  FFI session registry -> NativeHost -> LynxNativeRendererApiV1
                                             |
Lynx platform                                v
                                      Lynx Fiber DOM
```

The main boundaries are deliberately narrow:

1. **Framework adapters produce commands.** Yew and Dioxus translate their own
   renderer mutations into the same framework-neutral `CommandBatch`.
2. **The core owns correctness.** `Session` validates node ownership, tree shape,
   listener identity, mutation order, thread ownership, and deterministic
   teardown before commands reach Lynx.
3. **The FFI layer owns lifecycle safety.** It manages opaque session tokens,
   synchronous callbacks, reentry rejection, panic containment, poisoning after
   partial native failures, normal destroy, and emergency abandon.
4. **`NativeHost` owns the Lynx mapping.** It maps bridge IDs to opaque Lynx
   handles and executes each batch through a copied and validated
   `LynxNativeRendererApiV1` function table.
5. **Patched Lynx owns rendering.** The native renderer creates Fiber elements,
   applies mutations, delivers platform events, and flushes updates without a
   template or JavaScript runtime.

`CommandBatch` is an in-memory Rust boundary. Native applications do not
serialize their commands. Event content type and payload bytes are copied at the
native callback boundary but remain opaque to the bridge.

### Native And WASM Modes

Both runtime modes converge on the same `NativeHost` and Lynx C API:

```text
Native
  Yew/Dioxus app staticlib
      -> element-bridge-ffi
      -> NativeHost
      -> Lynx C API

WASM
  Yew/Dioxus wasm32-wasip1 guest
      -> Postcard guest ABI
      -> WAMR host
      -> element-bridge-ffi / NativeHost
      -> Lynx C API
```

Serialization exists only between a WASM guest and its WAMR host. The Android
APK contains the framework-neutral WAMR host, but no guest `.wasm`; guests are
built separately and loaded from a URL. Native and WASM sessions otherwise use
the same renderer lifecycle and safety model.

### Render And Event Flow

A mount or event follows one synchronous owner-thread transaction:

```text
framework render
  -> adapter records mutations
  -> Session validates and commits a CommandBatch
  -> NativeHost applies commands and flushes Lynx

platform event
  -> Lynx native callback
  -> FFI validates session/listener/callback identity
  -> adapter dispatches the framework callback
  -> framework renders the next CommandBatch
  -> NativeHost applies and flushes it
```

There is no rollback after Lynx has accepted part of a batch. A partial host
failure poisons the session so later calls cannot continue from an unknown
state. Normal destroy renders framework teardown mutations before releasing the
renderer; abandon only consumes bridge state and is reserved for emergency
cleanup.

### Repository Map

| Path | Responsibility |
| --- | --- |
| `crates/element-bridge-core/` | Framework-neutral IDs, commands, events, session invariants, and `HostFake` |
| `adapters/yew/` | Patched Yew `NativeRendererBackend` to core `Session` |
| `adapters/dioxus/` | Dioxus `WriteMutations` and Lynx-native `view`/`text` RSX vocabulary |
| `crates/adapter-conformance/` | Cross-framework mount, update, event, and destroy conformance |
| `crates/element-bridge-ffi/` | Native session registry, C ABI lifecycle, and `NativeHost` |
| `crates/element-bridge-wasm-guest/` | Versioned WASM guest ABI and Postcard protocol |
| `crates/element-bridge-wamr-host/` | WAMR embedding and guest-to-native backend integration |
| `adapters/android/` | JNI/CMake bridge that resolves the Lynx function table with `dlsym` |
| `examples/counter/` | Yew Native and WASM counter application |
| `examples/dioxus-counter/` | Dioxus Native and WASM counter application |
| `examples/android/` | Android launcher and Native/WASM runtime hosts |
| `tools/dev-wasm/` | WASM build, watch, HTTP serving, and reload notifications |
| `include/` | Public Lynx renderer, Native application, and WAMR application C ABIs |
| `patches/lynx/` | Ordered patch series implementing the Lynx native renderer |
| `patches/yew/` | Ordered patch series adding Yew's native renderer interface |

The effective Lynx and Yew integrations are the pinned upstream revisions plus
these patch series; they are not upstream APIs that can be assumed on arbitrary
versions.

## Getting Started

### 1. Run The Rust Tests

The repository pins Rust `1.85.0` in `rust-toolchain.toml`. Initialize the
submodules, prepare the patched Yew checkout, and run the host-side workspace
tests:

```bash
git submodule update --init --recursive
./scripts/bootstrap-yew.sh
cargo test --workspace --all-targets --locked
```

To include the real embedded WAMR lifecycle tests:

```bash
cargo test -p lynx-element-bridge-wamr-host --features wamr -- --test-threads=1
```

### 2. Build And Run The Android Example

The supported example target is Android API 24+ on `arm64-v8a`. The scripted
build requires:

- JDK 11.
- Android SDK platform 33 and build-tools 33.0.1.
- Android NDK 21.1.6352462 for Lynx and 25.2.9519653 for Rust/JNI linking.
- CMake 3.22.1.
- Rust 1.85.0 with `aarch64-linux-android` and `wasm32-wasip1`.
- Node.js 22.18.0 for Lynx source preparation.

Set `ANDROID_HOME` or `ANDROID_SDK_ROOT`, then build one Native framework
variant from the repository root:

```bash
export ANDROID_HOME=/path/to/Android/sdk
./scripts/build-android.sh --backend yew
# or
./scripts/build-android.sh --backend dioxus
```

The first online build initializes pinned sources, applies the patch series,
builds and publishes the local Lynx artifacts, links both Native and WAMR bridge
libraries, assembles the APK, and inspects its dependency and ELF boundaries.
Later prepared builds can add `--offline`; use `--clean` to discard generated
integration outputs. These two flags cannot be combined.

Install and open the selected APK:

```bash
adb install -r .deps/android/apks/lynx-element-bridge-yew.apk
adb shell am start -n com.yew.lynx.example/.LauncherActivity
```

For Dioxus, replace `yew` in the APK name with `dioxus`. The launcher lets you
open either the compiled Native counter or the external WASM flow. See
[`examples/android/README.md`](examples/android/README.md) for Android Studio,
offline build, and device acceptance details.

### 3. Run A WASM Guest

Build and serve both example guests with the repository's development server:

```bash
./scripts/dev-wasm.sh
adb reverse tcp:8000 tcp:8000
```

In the installed app, choose **WASM** and enter one of the URLs printed by the
server, for example:

```text
http://127.0.0.1:8000/yew_lynx_counter.wasm
http://127.0.0.1:8000/lynx_element_bridge_dioxus_counter.wasm
```

Use `./scripts/dev-wasm.sh --backend yew` or `--backend dioxus` to watch only
one guest. `--bind IP` and `--port PORT` change the listener. After a successful
rebuild, the app verifies the announced artifact and remounts it. Reload creates
a new component tree and does not preserve application state.

## Build Outputs

The Android build keeps the stock and native Lynx products separate:

| Product | Main library | Purpose |
| --- | --- | --- |
| `org.lynxsdk.lynx:lynx` | `liblynx.so` | Preserved stock JavaScript/template product |
| `org.lynxsdk.lynx:lynx-native-renderer` | `liblynx_native_renderer.so` | Opt-in native Fiber renderer used by this app |

Each example APK contains:

- `liblynx_element_bridge_native.so`, linked to the selected Yew or Dioxus
  Native runtime.
- `liblynx_element_bridge_wamr.so`, the framework-neutral WAMR host.
- `liblynx_native_renderer.so` and its native Lynx support libraries.

The build rejects stock `liblynx.so`, Quick/PrimJS, NAPI, V8, LynxJSSDK's
`assets/lynx_core.js`, packaged `.lynx.bundle` files, and packaged WASM guests.

## Verification

Run the complete repository verification after satisfying the Android build
requirements:

```bash
./scripts/verify.sh
```

It covers formatting, checks, tests, Clippy, adapter conformance, public native
lifecycle tests, real WAMR lifecycle tests, Android Java/JNI mocks, Rust Android
static libraries, Lynx and Yew patch application, public-header identity, and
artifact/dependency/ELF gates.

Pinned and currently verified inputs:

| Component | Version or revision |
| --- | --- |
| Lynx | `0df14207cebb060f1bed8de12b64a1119dee8f06` |
| Lynx tools_shared | `ff47fee7d41ee3e8e8561041b1ce2c8b50e923ea` |
| WAMR | `25bd7eb63e828e4bd242cc9b38d260b4b31c6605` |
| Yew patch base | `0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3` |
| Dioxus | `0.7.10` |
| Rust | `1.85.0` |

Do not infer compatibility across pin changes. See
[`COMPATIBILITY.md`](COMPATIBILITY.md) for the complete support matrix, runtime
contract, physical-device evidence, and known limits. See
[`docs/adapter-authoring.md`](docs/adapter-authoring.md) to add another Rust UI
framework adapter.

## Current Limits

- This is a research preview, not a production-ready runtime.
- Only Android API 24+ on `arm64-v8a` is currently supported and verified.
- iOS, Harmony, desktop, web, accessibility, and performance are not covered.
- Framework support is intentionally narrower than each framework's web
  renderer; unsupported Yew/Dioxus features must not be assumed to work.
- WASM reload replaces the complete guest/session and does not retain component
  state.

## Licensing

This repository is Apache-2.0. Upstream projects retain their own terms; see
[`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
