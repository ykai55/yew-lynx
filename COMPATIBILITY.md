# Compatibility and support status

Lynx Element Bridge is an experimental preview. Its support claim is limited to
the exact source revisions, native ABI, Android target, and evidence below.

## Matrix

| Component | Revision or target | Evidence |
| --- | --- | --- |
| Lynx | `0df14207cebb060f1bed8de12b64a1119dee8f06` | Pinned gitlink, sequential clean-apply 16-patch `0002-0017` series, public-header byte comparison, 22/22 native renderer tests |
| Lynx tools_shared | `ff47fee7d41ee3e8e8561041b1ce2c8b50e923ea` | Publicly reachable pinned nested checkout and clean-apply JNI-filter patch |
| Yew | `0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3` | Patch identity, renderer/macro tests, adapter, native lifecycle staticlib |
| Dioxus | `0.7.10` | Lynx-native RSX vocabulary, real `VirtualDom`, `WriteMutations` adapter, native lifecycle staticlib |
| Rust | `1.85.0` | Locked workspace format/check/test/Clippy and arm64 staticlibs |
| Android app | API 24+, tested arm64-v8a | Native Java/JNI lifecycle, both real staticlib links, dependency/APK/ELF and process-map checks |
| Native renderer ABI | `LynxNativeRendererApiV1`, version 1 | Size/version/function validation, opaque handles, callbacks, timers, compiled stylesheet import, release |
| WASM guest protocol | FlatBuffers guest ABI, version 4 (`LEB4`) | Schema and checked-in Rust bindings, golden fixtures, strict validation, and runtime-scoped mount, event, command, error, and teardown round trips |

## Runtime Contract

- `CommandBatch` is an ordered in-memory `Vec<Command>` mutation boundary.
- Native registry session tokens remain nonzero opaque lifecycle handles, but
  are not carried by `CommandBatch`, `EventMessage`, or WASM protocol messages.
- Nodes, listeners, callbacks, and native handles are nonzero opaque IDs scoped
  by their owning backend and host objects.
- Session operations, function-table calls, and callbacks stay synchronous on
  the mounting thread; wrong-thread and reentrant calls are rejected.
- Session tree ownership and exact listener identity are validated before each
  mutation.
- Native event content type and payload bytes remain opaque to the bridge.
- Host application is ordered and does not promise rollback after a partial
  native failure; such a failure poisons the session.
- Normal destroy removes framework state and then releases the native renderer.
  Abandon skips application teardown and exists only for emergency cleanup.
- Android registration rejects a host with an active BTS runtime, MTS context,
  or template lifecycle.

There is no serialized application command/result channel, runtime capability
negotiation, Java module transport, or template-bundle application path.

## Android Artifact Contract

The stock and native products are deliberately separate at the same pinned
version:

| Maven coordinate | Shared library | Contract |
| --- | --- | --- |
| `org.lynxsdk.lynx:lynx:0.0.1-0df14207` | `liblynx.so` | Preserved stock JavaScript/template product, built and published without a behavior change |
| `org.lynxsdk.lynx:lynx-native-renderer:0.0.1-0df14207` | `liblynx_native_renderer.so` | Opt-in native renderer product used by the example app |

The native product and app dependency graphs exclude stock `lynx`, Quick and
PrimJS, NAPI, V8, LynxJSSDK, and standalone Wasm runtime artifacts. The APK
excludes `liblynx.so`, runtime engine shared libraries, and `assets/lynx_core.js`.
The app packages separate Native and WAMR bridge libraries. The Native Android
build variant selects Yew or Dioxus; the WAMR host accepts an externally built
compatible `wasm32-wasip1` guest from a URL. No guest `.wasm` is packaged.
ELF inspection rejects forbidden runtime
`DT_NEEDED` entries and undefined runtime symbols, and requires the public
`lynx_native_renderer_get_api` export. PrimJS preparation is
retained solely to reproducibly build and test the stock artifact; it is not a
native product or application dependency.

## Final Device Evidence

Both frameworks passed the final patch-0015 binary-native physical-device
acceptance flow on 2026-08-22:

- Device: anonymous Xiaomi Redmi K60 Pro physical device
- OS: Android 13, API 33
- ABI: arm64-v8a
- Repeated cycles: 3
- Yew totals: `onCreate=9`, `onDestroy=4`, `diagnostics=9`, selected backend 9
- Dioxus totals: `onCreate=9`, `onDestroy=4`, `diagnostics=9`, selected backend 9
- Functional result flags: all true (fresh launch, tap visual change, activity
  recreation, and force-stop/reopen) for both backends
- Diagnostics: `renderer_mode=native`, `bts_runtime=false`,
  `mts_context=false`, `template=false`
- Process maps: `proc_maps_checked=true`; all five required libraries mapped
  after fresh launch and interaction, forbidden library set empty, and
  Quick/NAPI/Wasm/V8 flags false
- Crash markers: 0 per run

| Backend | APK SHA-256 | Evidence path |
| --- | --- | --- |
| Yew | `121c20a6bc82d1570eb24cfb37c84a5d82c414bc1b176c4b40ac8294fe45903d` | `.deps/android/device-acceptance-binary-native-yew-20260822-0015-final` |
| Dioxus | `008d64415874bff997a47c1afcb25dc2e87c5fe4e6f513acaad2bf8273f3afdc` | `.deps/android/device-acceptance-binary-native-dioxus-20260822-0015-final` |

The current acceptance flow writes eight files: four screenshots, `logcat.txt`,
`maps-fresh.txt`, `maps-after-interaction.txt`, and `summary.json`.

## Limits

No claim is made for other Lynx, Yew, or Dioxus revisions; Android ABIs other
than arm64-v8a; Android API levels outside the declared app range; iOS, Harmony,
desktop, or web; accessibility; performance; asynchronous Dioxus scheduling; or
production use.

The 21 `NativeRendererApiTest` cases pass (21/21). They use no private test peer
or helper, and release behavior is tested through the production function table.
All 11 public statuses cross the production boundary. `RESOURCE_EXHAUSTED` and
`INTERNAL_ERROR` are returned by real repeating timer callbacks through the
function table, after which production task-runner behavior cancels each timer.
Neither status uses allocator exhaustion or root rollback. Issue #4's
implementation and acceptance requirements are complete, and the issue is
closable.

A Lynx pin change requires rebasing 0002-0016 and rerunning patch, header,
native-host, product, ELF, Android, and device verification. A tools_shared pin
change requires rebasing and reverifying its JNI-filter patch. A Yew pin change
requires rebasing its patch and focused tests. A Dioxus pin change requires the
real `VirtualDom` and cross-framework conformance suites.
