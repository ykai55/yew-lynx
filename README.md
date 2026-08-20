# yew-lynx

> [!WARNING]
> **Experimental public preview.** This is an independent research prototype,
> not an officially supported Yew or Lynx integration. It provides the source
> adapters, a standalone Android example, and the pinned build and device
> evidence described below. Compatibility must not be generalized beyond those
> exact inputs.

This repository implements an ordinary stock OSS Lynx integration path:

```text
patched Yew native_renderer
  -> Rust validated protocol v1 and counter staticlib
  -> Android public LynxModule and JNI UTF-8 byte[] bridge
  -> synchronous MTS broker
  -> ordinary context-type 1 LepusNG/Fiber template
  -> stock Lynx renderer
```

The Lynx API audit is pinned to
[`0df14207cebb060f1bed8de12b64a1119dee8f06`](https://github.com/lynx-family/lynx/tree/0df14207cebb060f1bed8de12b64a1119dee8f06).
The path uses the revision's public typed
[Fiber Element globals](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/js_libraries/type-element-api/types/element-api.d.ts)
and public Android
[`LynxModule`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/jsbridge/LynxModule.java)
surface. Stock LepusNG registers `lynx.module()` in its
[renderer bindings](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/runtime/lepusng/bindings/renderer_ng.cc),
but `module()` is absent from that revision's declared public
[main-thread `Lynx` TypeScript interface](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/js_libraries/types/types/main-thread/lynx.d.ts).
It is therefore a revision-pinned integration surface, not a stable declared
TypeScript API.

`third_party/lynx` pins that exact public revision as a source submodule. The
integration applies no Lynx patch and makes no direct JNI call into hidden stock
Lynx C++ symbols.

## Status

| Area | Status |
| --- | --- |
| Yew base | Pinned to exactly [`0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3`](https://github.com/yewstack/yew/tree/0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3) |
| Yew patch | Experimental `native_renderer` feature with focused renderer and macro tests |
| Rust bridge | Protocol-v1 mutation validation, counter lifecycle, C ABI, and `staticlib` verified by host tests |
| MTS/Fiber bridge | Broker tests plus an encoded and decoded ordinary LepusNG bundle |
| Android bridge | Public Kotlin host, `LynxModule`, real JNI/NDK link, and arm64 APK built against locally published stock Lynx AARs |
| Stock Lynx runtime/device | Pinned APK accepted on one Dora Android 15 / API 35 / `arm64-v8a` physical device; not a broader device claim |
| Support level | Experimental preview with no official support or stability commitment |

## Included

- `patches/yew/`: a format-patch series adding Yew's opt-in
  `native_renderer` API.
- `crates/native-renderer-adapter/`: strict protocol-v1 envelopes and a
  validated mutation recorder.
- `examples/counter/`: a Rust `staticlib` counter and public C ABI for mount,
  tap dispatch, destroy, and response-buffer ownership.
- `adapters/mts/`: the synchronous MTS broker, public Fiber host, ordinary
  LepusNG shell, bundle build, and host-independent tests.
- `adapters/android/`: a public `LynxModule`, JNI bridge, Android integration
  sources, and Java/JNI mock checks.
- `third_party/lynx/`: the exact public Lynx source submodule used to build the
  local Android AAR repository, without source patches.
- `examples/android/`: the standalone Kotlin/Gradle Kotlin DSL host that links
  the real Rust archive and loads the generated bundle in a `LynxView`.
- `android/` and `scripts/build-android.sh`: locked Habitat/PrimJS inputs and the
  clean, cached, and offline-capable Android orchestration.
- `scripts/bootstrap-yew.sh`: reproducible bootstrap for the exact Yew base and
  patch series.
- `scripts/verify.sh`: the local and CI verification entry point.

## Evidence boundary

The normal verification workflow remains the fast source and host-test seam:

- Rust tests cover strict protocol decoding, positive JavaScript-safe IDs,
  mutation ownership, initial counter mount, one tap update, thread ownership,
  explicit destroy, stale handles, and panic-to-error boundaries.
- Patched-Yew tests cover the narrow renderer surface, synchronous lifecycle,
  direct-parent teardown, selected pre-mutation rejection, unwind cleanup, and
  typed `ontap` macro behavior. They also reject mounting another native
  renderer while Yew's scheduler is already executing and verify emergency
  state abandonment without host mutation.
- `npm run build` creates an ordinary LepusNG template bundle and decodes it to
  verify `context-type` 1 and `is-lepusng-binary: true`; broker tests use mock
  Element globals and native modules.
- Android checks compile and run the Java lifecycle/schema tests, compile the
  real JNI source against a mock Rust implementation, run a JNI round trip, and
  syntax-check the JNI source against the repository C header. A separate C
  smoke program links and calls the real host Rust `staticlib`, and verification
  also builds the Android arm64 archive.

The separate Android integration workflow builds the six required AARs from the
pinned stock Lynx submodule, publishes them locally, links the real Rust/JNI
shared library, assembles the app, repeats assembly offline, checks APK ABI and
native-library contents, and uploads the APK plus build evidence. An
authenticated local Dora run additionally proved initial `Count: 0`, a real
tap to `Count: 1`, Activity recreation, force-stop/reopen reset, and repeated
mount/tap/destroy cycles on one Android 15 ARM64 physical device. Screenshots
and device connection details remain ignored local evidence. See
[COMPATIBILITY.md](COMPATIBILITY.md) for the exact claim boundary.

## Protocol and lifecycle bounds

- Protocol v1 accepts only positive IDs up to `Number.MAX_SAFE_INTEGER`
  (`9007199254740991`). IDs cross MTS and Java as decimal `String` values and
  cross JNI as UTF-8 bytes.
- Every response has an exact success or failure envelope with no unknown
  fields. Every successful batch, including a no-op, ends in exactly one final
  `flush`. Initial mount validates but suppresses that flush because the outer
  Lynx render pipeline flushes after `__RenderPage` returns.
- The only event is `tap`; protocol v1 has no event payload, propagation, or
  asynchronous dispatch.
- The Lynx adapter accepts raw text only beneath a `<text>` element. Other Yew
  native backends may define a different text-node ownership rule.
- One `YewLynxModule` owns at most one live Rust session. All calls for that
  session run synchronously on its mounting thread.
- Cached `initPage` roots, nonempty cache data, and SSR hydration roots are
  rejected. Reload destroys and remounts; component removal destroys and
  permits a later fresh mount.
- Teardown is explicit. Hosts must route removal, reload, lifetime destruction,
  and module destruction to the broker/module destroy path.
- `NativeRenderer::render()` is rejected from inside a running Yew scheduler
  callback. If a host thread exits without explicit destroy, Rust abandons its
  local state to break reference cycles, but it cannot clean the host tree.
- Panic containment depends on unwinding. `panic = "abort"` cannot run cleanup
  guards, and a second panic from a backend during unwinding can abort. Backend
  methods and callbacks must not panic.

## Android host responsibilities

A consuming Android host must:

1. Use a stock OSS Lynx build compatible with the audited APIs.
2. Enable MTS modules with `setEnableMTSModule(true)`.
3. Register `YewLynxModule` per runtime, not as a shared module.
4. Build the Rust counter archive for each packaged Android ABI and link it into
   `libyew_lynx_bridge.so`.
5. Package and load the JNI shared library.
6. Load `adapters/mts/dist/yew-lynx-counter.lynx.bundle` through the normal
   `LynxView` template-loading path.
7. Preserve same-owner-thread synchronous calls and explicit teardown.

`examples/android` implements these responsibilities for
`aarch64-linux-android` / `arm64-v8a` only and limits `abiFilters`
accordingly. Other hosts and ABIs remain untested.

## Prerequisites

- Git with network access to <https://github.com/yewstack/yew>
- Rust 1.85.0 with `rustfmt` and `clippy`, as declared in
  `rust-toolchain.toml`
- Node.js 22.18.0 (pinned by `.nvmrc`) and npm
- Bash, a JDK, and a C++17 compiler for Android mock checks
- For the standalone APK: JDK 11, Android platform/build-tools 33/33.0.1,
  NDKs 21.1.6352462 and 25.2.9519653, and CMake 3.22.1
- Optional ShellCheck, which `scripts/verify.sh` runs when available

## Bootstrap and build

From the repository root:

```bash
./scripts/bootstrap-yew.sh
npm --prefix adapters/mts ci
npm --prefix adapters/mts run build
```

To serve the generated bundle through a local HTTP service for a Lynx host or
Explorer, run:

```bash
npm --prefix adapters/mts run serve
```

The served bundle URL is:

```text
http://127.0.0.1:4173/yew-lynx-counter.lynx.bundle
```

The command prints a terminal QR code for the served template URL. Use
`-- --no-qr` or `NO_QR=1` to suppress QR output.

Use `npm --prefix adapters/mts run serve -- --host 0.0.0.0` when a physical
device needs to fetch the bundle from the development machine.

For the complete Android integration, initialize the toolchains listed above
and run the single orchestration entry point:

```bash
./scripts/build-android.sh
```

It initializes the pinned Lynx submodule, bootstraps patched Yew, verifies and
materializes locked Habitat and PrimJS bytes, builds and publishes local stock
Lynx AARs, links the real arm64 bridge, assembles the APK, and repeats app
assembly with Gradle, npm, and Cargo offline. `--clean` discards generated
Android outputs; `--offline` requires a matching cache prepared by a successful
online build. See [`examples/android/README.md`](examples/android/README.md).

The template build uses exact development dependencies `esbuild` 0.25.9 and
`@lynx-js/tasm` 0.0.51. Local QR output uses `qrcode-terminal` 0.12.0. The
build emits these ignored files:

```text
adapters/mts/dist/shell.js
adapters/mts/dist/template-input.json
adapters/mts/dist/yew-lynx-counter.lynx.bundle
```

## Verify

```bash
./scripts/verify.sh
```

Verification performs:

1. Shell syntax checks and optional ShellCheck.
2. Lynx gitlink/submodule URL and Android lock-metadata validation.
3. Idempotent Yew bootstrap and patch identity validation.
4. `cargo fmt --check`, workspace `cargo check`, `cargo test`, and
   `cargo clippy -D warnings` with locked dependencies.
5. `npm ci`, `npm run build`, and `npm test` for the ordinary LepusNG/MTS
   template and broker, plus a forced WASM codec build.
6. Android Java lifecycle/schema and JNI mock integration checks, a real host
   C/staticlib ABI smoke test, and an Android arm64 staticlib build.
7. Patched-Yew `native_renderer` checks and focused renderer tests.
8. Yew macro tests with and without `native_renderer`.

## Licensing

This repository is licensed under the [Apache License 2.0](LICENSE). Yew is
available under MIT or Apache-2.0. Lynx, PrimJS, Habitat, and the downloaded
`@lynx-js/tasm` and `qrcode-terminal` development packages are Apache-2.0;
esbuild is MIT. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
