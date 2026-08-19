# Compatibility and support status

This experimental preview implements and builds an ordinary stock OSS Lynx
Android path. Its compatibility claim stops at the exact source, dependency,
APK, and single-device evidence listed here.

## Compatibility matrix

| Component | Revision or target | Status | Evidence |
| --- | --- | --- | --- |
| Yew | `0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3` | Exact patch base only | Bootstrap validates patch identities; focused renderer and macro tests run against the patched checkout |
| Yew at another revision | Unpinned | Not evaluated | Rebase and rerun all checks before changing the pin |
| Rust | 1.85.0 | Repository verification toolchain | `rust-toolchain.toml`, locked workspace checks, tests, formatting, and clippy |
| Protocol | Version 1 | Implemented for the documented mutation and lifecycle surface | Rust protocol/backend tests, counter C ABI tests, MTS broker tests, and Android schema tests |
| Template toolchain | `esbuild` 0.25.9 and `@lynx-js/tasm` 0.0.51 | Exact npm development dependencies | `npm ci`; build emits and decodes the bundle as context-type 1/LepusNG |
| OSS Lynx APIs | `0df14207cebb060f1bed8de12b64a1119dee8f06` | Audited source target for the ordinary MTS/Fiber route | Pinned upstream citations and [`docs/oss-lynx-gap.md`](docs/oss-lynx-gap.md) |
| Android adapter | Public `LynxModule` plus JNI | Source, mock, and real arm64 NDK link verified | Java lifecycle/schema checks, JNI round trip, host C ABI smoke test, and packaged `libyew_lynx_bridge.so` |
| Standalone APK | API 24+, `arm64-v8a` only | Built against locally published AARs from the exact Lynx pin | Online assembly, offline reassembly, strict dependency verification, APK content checks |
| Physical device | Android 15 / API 35, `arm64-v8a` | One Dora acceptance run only | `Count: 0`, tap to `Count: 1`, recreation/reset, and repeated lifecycle screenshots/logs |

## Implemented route

```text
Yew native_renderer patch
  -> Rust protocol-v1 recorder and counter staticlib
  -> Android LynxModule + JNI UTF-8 byte[]
  -> synchronous MTS module proxy
  -> public typed Fiber Element globals
  -> ordinary LepusNG/Fiber bundle
  -> stock renderer
```

The pinned Lynx revision declares the
[Fiber Element globals](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/js_libraries/type-element-api/types/element-api.d.ts),
registers them for Fiber and registers `lynx.module()` in the
[LepusNG renderer bindings](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/runtime/lepusng/bindings/renderer_ng.cc).
The same revision's public
[`Lynx` TypeScript interface](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/js_libraries/types/types/main-thread/lynx.d.ts)
does not declare `module()`. This project therefore treats it as pinned stock
implementation behavior, not a stable declared TypeScript contract.

The exact Lynx source checkout is included as `third_party/lynx`; no Lynx patch
is applied. The route does not bind hidden Lynx C++ symbols, and direct JNI
calls to those symbols remain unsupported.

## Verified behavior

| Layer | Evidence and boundary |
| --- | --- |
| Patched Yew | Top-level component, text/tags/lists, initial attributes, synchronous `rendered` and destroy lifecycle, direct-parent teardown, one `ontap` update, selected pre-mutation rejection, and unwind cleanup for a panicking destroy callback |
| Rust adapter | Exact protocol envelopes, JavaScript-safe IDs, ownership validation, one final flush, no-op flush, tap dispatch, stale listener handling, poisoned sessions, and C ABI buffer ownership |
| Counter staticlib | Initial `Count: 0`, one tap update to `Count: 1`, explicit cleanup, same-thread enforcement, and stale-session rejection in host tests |
| MTS broker | Exact JSON/schema validation before host mutation, public Fiber operation mapping, listener ownership, initial flush suppression, explicit destroy fallback, reload/removal lifecycle, and cache/SSR rejection against mocks |
| Template build | Bundled MTS shell plus N-API and forced-WASM codec builds, each decoded as `context-type` 1 and `is-lepusng-binary: true` |
| Android adapter | Public-module shape, synchronous string calls, UTF-8 `byte[]` JNI transport, one live session, remount after `destroySession()`, permanent module teardown, JNI allocation/cleanup paths against mocks, a real host C/staticlib ABI smoke test, and an Android arm64 archive build |
| Standalone Android | Six local stock Lynx AARs, real Rust/JNI shared-library link, ordinary bundle asset load, arm64-only APK, online/offline app assembly, physical-device tap and lifecycle evidence |

This evidence does not establish other Lynx revisions, Android versions,
devices, ABIs, accessibility conformance, performance, or production support.

## Protocol and lifecycle contract

- Node, listener, root, and session IDs are positive and no greater than
  `9007199254740991`. Root and listener IDs pass through MTS and Java as decimal
  strings, avoiding lossy JavaScript number conversion at the Java boundary.
- Success and failure envelopes are exact and reject unknown fields. Every
  success response, including a no-op, has exactly one final `flush`. A failure
  normally has no operations; destroy alone may return a validated partial
  cleanup sequence ending in one flush.
- Initial mount validates the final flush but suppresses its execution because
  Lynx's enclosing render pipeline flushes after `__RenderPage` returns. Event,
  update remount, explicit batch, and destroy paths retain their flush boundary.
- Protocol v1 supports `tap` only. Event payloads, propagation data,
  asynchronous calls, nested dispatch, and reentrant broker operations are out
  of scope.
- Raw text must be attached directly beneath a `<text>` element for this Lynx
  adapter. This is stricter than Yew's generic native renderer contract.
- A module instance owns at most one live Rust session. Every call is
  synchronous and must execute on the mounting thread.
- Cached `initPage` roots, nonempty cache data, and SSR hydration are rejected.
  Reload destroys then remounts; component removal destroys and allows a later
  fresh mount.
- Teardown is explicit through broker/module lifecycle paths. Dropping a Rust
  application handle or exiting its owner thread does not perform host cleanup.
- A new `NativeRenderer::render()` call is rejected while Yew's scheduler is
  already running. `NativeAppHandle::destroy(&mut self)` returns a busy error in
  the same state without consuming the handle, so teardown can be retried.
  Owner-thread exit abandons local Rust state to break cycles, but still cannot
  remove host nodes or callbacks.
- Rust catches unwind panics at exported C boundaries, but `panic = "abort"`
  cannot be contained. Backend panics during unwind can still abort, so backend
  operations and callbacks must be non-panicking.

## Yew renderer boundary

The current patch intentionally supports only a top-level component with
`VNode::VText`, authored `VNode::VTag`, source-order `VNode::VList`, initial
attributes/`value`/`checked`, and `ontap`. Nested components, explicit
`NodeRef`, keys, portals, suspense, browser references, raw HTML, other events,
and fine-grained in-place text or attribute updates are not supported.

See [`patches/yew/README.md`](patches/yew/README.md) for the exact API and test
commands.

## Android host boundary

The included host enables MTS modules, registers one `YewLynxModule` per
runtime, cross-compiles and packages the Rust archive through the JNI shared
library, loads the generated bundle through a normal `LynxView`, keeps calls on
the UI owner thread, and explicitly destroys the view. The claim applies only
to the pinned Lynx revision, locked dependencies, arm64 APK, and recorded
Android 15 device run.

## Changing a pin

A Yew revision change requires rebasing `patches/yew/series`, validating patch
identity from a clean checkout, and rerunning `scripts/verify.sh`. A Lynx
revision change requires a new audit of the public Fiber globals, LepusNG module
binding, Android module APIs, MTS opt-in, and template compatibility. A newer
revision must not be assumed compatible.
