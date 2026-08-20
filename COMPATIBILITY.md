# Compatibility and support status

Lynx Element Bridge is an experimental preview. Its compatibility claim stops
at the exact revisions, source generation, and host verification listed here.

## Matrix

| Component | Revision or target | Evidence |
| --- | --- | --- |
| Protocol | FlatBuffers v2, identifier `LEB2` | Rust round trips, TypeScript consumer tests, Java/JNI binary transport tests |
| FlatBuffers | `25.2.10` | Locked compiler download and runtime dependencies; committed generated Rust/TypeScript/Java |
| Lynx | `0df14207cebb060f1bed8de12b64a1119dee8f06` | Pinned submodule, generated 107-declaration manifest, clean-apply ByteArray patch gate |
| Yew | `0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3` | Patch identity checks, renderer/macro tests, adapter and real counter tests |
| Dioxus Core | `0.7.10` | `WriteMutations` adapter, exact listener/callback registry, real `VirtualDom` wire/FFI counter tests, arm64 staticlib and APK assembly |
| Rust | `1.85.0` | Locked workspace check, test, formatting, Clippy, host and arm64 builds |
| Android | API 24+, `arm64-v8a` reference target | Framework-neutral Java/JNI mocks, dual real-staticlib links, isolated Yew and Dioxus APK builds |
| MTS toolchain | Node 22.18.0, esbuild 0.25.9, `@lynx-js/tasm` 0.0.51 | Locked install, native and forced-WASM bundle builds, broker tests |

## Capability Surface

The schema contains one typed table for each of the 107 public declarations in
the pinned Lynx Element API type package. The revision manifest reports 100 as
available on Android and 7 as unsupported. The 33 native-only registry bindings
that are absent from the public type package are out of scope.

Capability support is revision metadata, not a runtime probe. Required gaps
reject session creation; optional gaps return `UNSUPPORTED` for the associated
result slot.

## Runtime Contract

- IDs are nonzero opaque 32-bit values scoped to one session.
- Calls are synchronous on the mounting thread; cross-thread calls fail with
  `WRONG_THREAD`.
- Commands, results, and events use distinct FlatBuffers channels.
- Events retain opaque payload bytes and a content type.
- The MTS consumer validates a complete command batch before host mutation.
- Host execution is ordered and has no transactional rollback guarantee.
- Explicit destroy invalidates all IDs and releases tree/listener state.
- Android Java returns `byte[]`; the pinned Lynx patch exposes it to ordinary
  LepusNG as a read-only `length` plus numeric-index byte view.

## Evidence Boundary

Host-independent verification covers both framework adapters, the core host
fake, protocol generation, FlatBuffers verification, MTS/Fiber mocks, the
framework-neutral Android Java/JNI transport, both real host static libraries,
and isolated arm64 APK assembly for Yew and Dioxus.

The acceptance script accepts `--backend yew|dioxus`, retains visual counter and
lifecycle checks, and rejects logs that do not identify the expected linked
backend.

On 2026-08-20, the v2 Yew counter passed that flow on a Samsung SM-S9210 running
Android 15/API 35 with `arm64-v8a`. Evidence covered initial `Count: 0`, tap to
`Count: 1`, rotation recreation, force-stop/reopen, and three repeated
mount/tap/destroy cycles; the run observed 9 Activity creates and 4 destroys.
The tested APK SHA-256 was
`cff39b46c2b2bfc5b6d0f428229adb60818232c8d57b2e21ccc80a216be56925`.

On 2026-08-20, the Dioxus counter independently passed the same flow on a
Samsung SM-S9310 running Android 15/API 35 with `arm64-v8a`. Screenshots were
reviewed for exact `Count: 0` and `Count: 1` states; the run observed 9 Activity
creates and 4 destroys, and logcat repeatedly identified `backend=dioxus` with
no crash or native bridge failure. The tested APK SHA-256 was
`5bf66c4c6aab42af3af900ccc7277d02d369532ef1729029ceb7e144988fc430`.

No claim is made for other Lynx revisions, Android ABIs, iOS, Harmony, desktop,
web, accessibility, performance, Dioxus asynchronous scheduling, or production
use.

## Changing Pins

A Lynx revision change requires regenerating and reviewing the schema and
manifest, rebasing `patches/lynx`, and rerunning all verification. A Yew revision
change requires rebasing `patches/yew`. A Dioxus change requires rerunning the
real `VirtualDom` fixture and conformance suite. Compatibility must not be
inferred across any of these changes.
