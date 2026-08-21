# Compatibility and support status

Lynx Element Bridge is an experimental preview. Its support claim is limited to
the exact source revisions, native ABI, Android target, and evidence below.

## Matrix

| Component | Revision or target | Evidence |
| --- | --- | --- |
| Lynx | `0df14207cebb060f1bed8de12b64a1119dee8f06` | Pinned gitlink, sequential clean-apply 0002-0009 series, public-header byte comparison, native renderer tests |
| Yew | `0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3` | Patch identity, renderer/macro tests, adapter, native lifecycle staticlib |
| Dioxus Core | `0.7.10` | `WriteMutations` adapter, real `VirtualDom`, native lifecycle staticlib |
| Rust | `1.85.0` | Locked workspace format/check/test/Clippy and arm64 staticlibs |
| Android app | API 24+, tested arm64-v8a | Native Java/JNI lifecycle, both real staticlib links, native-only APK checks |
| Native renderer ABI | `LynxNativeRendererApiV1`, version 1 | Size/version/function validation, opaque handles, callbacks, timers, release |

## Runtime Contract

- `CommandBatch` is an ordered in-memory `Vec<Command>` mutation boundary.
- Sessions, nodes, listeners, callbacks, and native handles are nonzero opaque
  IDs scoped to one session.
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

## Final Device Evidence

Both frameworks passed the final patch-0009 release-safe physical-device
acceptance flow on 2026-08-22:

- Device: anonymous physical OPPO PGBM10 device
- OS: Android 13, API 33
- ABI: arm64-v8a
- Repeated cycles: 3
- Yew totals: `onCreate=9`, `onDestroy=4`, `diagnostics=9`, selected backend 9
- Dioxus totals: `onCreate=10`, `onDestroy=5`, `diagnostics=10`, selected backend
  10 because the OS performed one extra recreation
- Acceptance result flags: all six true (fresh launch, timer visual change, tap
  visual change, activity recreation, force-stop/reopen, and visual review) for
  both backends
- Diagnostics: `renderer_mode=native`, `bts_runtime=false`,
  `mts_context=false`, `template=false`
- Rejected log markers per run: wrong backend 0; crash 0; timer teardown 0

| Backend | APK SHA-256 | Evidence path |
| --- | --- | --- |
| Yew | `685e8000ac037607fc9cd870d0445293c3f11ec11d6c00b2a375033ed468a1bf` | `.deps/android/device-acceptance-native-yew-20260822-release-safe-success` |
| Dioxus | `6a51be912a01764566edbf8bea859effe0af073116a785fd6251d71f53664513` | `.deps/android/device-acceptance-native-dioxus-20260822-release-safe-success` |

The diagnostics prove only which rendering path executed. The stock Lynx AAR
still packages and loads Quick, PrimJS, and NAPI. Binary-native packaging remains
a blocked follow-up milestone; complete JS-engine removal is not claimed.

## Limits

No claim is made for other Lynx, Yew, or Dioxus revisions; Android ABIs other
than arm64-v8a; Android API levels outside the declared app range; iOS, Harmony,
desktop, or web; accessibility; performance; asynchronous Dioxus scheduling; or
production use.

A Lynx pin change requires rebasing 0002-0009 and rerunning patch, header,
native-host, Android, and device verification. A Yew pin change requires
rebasing its patch and focused tests. A Dioxus pin change requires the real
`VirtualDom` and cross-framework conformance suites.
