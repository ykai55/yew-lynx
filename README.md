# Lynx Element Bridge

> [!WARNING]
> **Experimental public preview.** This is an independent research project,
> not an officially supported Lynx, Yew, or Dioxus integration. Compatibility
> is limited to the pinned revisions and verification described here.

Lynx Element Bridge is a framework-neutral SDK for driving Lynx's public
Element API from native UI frameworks. Framework adapters emit ordered command
batches; the core owns sessions, capabilities, opaque IDs, validation, results,
events, and teardown; Android and MTS provide the reference host.

```text
Yew native_renderer        Dioxus WriteMutations
          \                    /
           framework-neutral core
                    |
        FlatBuffers v2 (LEB2 ByteArray)
                    |
        Android LynxModule/JNI <-> MTS broker
                    |
        pinned public Lynx Element API
```

## Pinned Inputs

- Lynx: `0df14207cebb060f1bed8de12b64a1119dee8f06`
- Yew patch base: `0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3`
- Dioxus Core: `0.7.10`
- FlatBuffers compiler/runtime: `25.2.10`
- Rust: `1.85.0`

`third_party/lynx` pins the audited Lynx source. `patches/lynx` contains one
minimal read-only ByteArray patch needed for ordinary LepusNG/MTS to consume
the binary protocol without Base64 or string conversion. `patches/yew`
contains the host-independent native renderer patch.

## Status

| Area | Current state |
| --- | --- |
| Protocol | FlatBuffers v2 with `LEB2` identifier and committed Rust, TypeScript, and Java generated code |
| Element API | 107 typed command tables generated from the pinned declaration package |
| Capabilities | 100 Android capabilities available; 7 gaps reported as `UNSUPPORTED` |
| Core | Session negotiation, owner-thread enforcement, opaque 32-bit IDs, ordered batches, Result/Event channels, host fake, and deterministic destroy |
| Yew | Real patched `NativeRenderer` adapter and Android counter staticlib |
| Dioxus | Real `dioxus-core` `VirtualDom` counter through the `WriteMutations` adapter |
| MTS | ByteArray verifier/decoder, typed Element dispatcher, Fiber host, Result return path, and lifecycle broker |
| Android | Numeric ID and `byte[]` Java/JNI boundary with Java, JNI, and real staticlib host checks |
| Lynx patch | Clean-apply gate plus focused upstream-style LepusNG ByteArray test |

The earlier protocol-v1 JSON recorder has been removed. The v2 Yew counter has
passed the repository's physical-device acceptance flow on an Android 15/API 35
arm64 device, including mount, tap, recreation, force-stop/reopen, and repeated
teardown. Dioxus device acceptance has not yet been run.

## Repository Layout

- `crates/element-bridge-core/`: framework-neutral domain model, validation,
  capability negotiation, result/event types, and in-memory host fake.
- `crates/element-bridge-wire/`: verified FlatBuffers v2 encoding and decoding.
- `adapters/yew/`: patched Yew `NativeRendererBackend` adapter.
- `adapters/dioxus/`: Dioxus 0.7.10 `WriteMutations` adapter.
- `crates/adapter-conformance/`: identical Yew/Dioxus mount, event, query,
  capability-gap, and destroy scenarios.
- `examples/counter/`: Yew counter staticlib and Android C ABI.
- `examples/dioxus-counter/`: real Dioxus `VirtualDom` counter fixture.
- `adapters/mts/`: protocol reader/writer, typed dispatcher, Fiber host, shell,
  and mock-host tests.
- `adapters/android/`: public `LynxModule`, JNI bridge, and integration checks.
- `protocol/`: schema, revision capability manifest, lock, and generated code.
- `patches/lynx/`: pinned ByteArray patch and focused runtime test.
- `patches/yew/`: pinned native renderer patch.
- `examples/android/`: standalone Android reference host.

See [`docs/adapter-authoring.md`](docs/adapter-authoring.md) to add another UI
framework.

## Protocol Contract

- All sessions, nodes, listeners, and callbacks use nonzero opaque `u32` IDs.
- Calls are synchronous and remain on the session's mounting thread.
- A `CommandBatch` is ordered and ends at one final commit boundary.
- Scalar and query returns use `ResponseBatch` on the Result channel.
- Host callbacks use `EventMessage` with an opaque byte payload and content
  type; the bridge does not interpret framework semantics.
- Missing required capabilities fail session creation. Missing optional
  capabilities produce an item-level `UNSUPPORTED` result.
- Batches are validated before host mutation. Host failures do not imply
  transactional rollback of mutations already applied.
- Destroy releases listeners, descendants, host references, and adapter state;
  stale IDs are rejected.

The capability manifest is generated from Lynx's pinned public
`element-api.d.ts`. The seven currently unsupported Android declarations are
`__SetStaticStyle`, `__CreateGestureDetector`, `__GeneratePipelineOptions`,
`__OnPipelineStart`, `__BindPipelineIDWithTimingFlag`, `__MarkTiming`, and
`__AddTimingListener`.

## Bootstrap

```bash
./scripts/bootstrap-yew.sh
./scripts/prepare-flatc.sh
npm --prefix adapters/mts ci
node scripts/generate-protocol.mjs
```

`scripts/generate-protocol.mjs` is the single source-generation entry point.
It reads the pinned Lynx declarations and emits the schema, capability manifest,
typed command tables, and language bindings. Generated files are committed and
CI requires regeneration to produce no diff.

## Verify

```bash
./scripts/verify.sh
```

The verification entry point checks:

1. Shell syntax, pins, lock metadata, and clean application of both patch series.
2. Protocol regeneration with locked `flatc` and committed-output consistency.
3. Workspace formatting, checks, tests, and Clippy with locked dependencies.
4. The shared Yew/Dioxus conformance scenario and real framework counters.
5. MTS bundle builds, ByteArray/FlatBuffers decoding, typed dispatch, Result
   return, lifecycle, and mock Fiber behavior.
6. Android Java lifecycle/schema checks, JNI binary round trips, a real host
   staticlib smoke test, and Android arm64 Rust compilation.
7. Focused patched-Yew renderer and macro tests.

For the complete Android build, use `./scripts/build-android.sh`. It temporarily
applies the pinned Lynx patch to a verified clean submodule, builds the required
AARs, and removes the patch on exit. JDK 11, Android SDK 33, build-tools 33.0.1,
NDKs 21.1.6352462 and 25.2.9519653, and CMake 3.22.1 are required.

## Compatibility

See [`COMPATIBILITY.md`](COMPATIBILITY.md) for the exact support and evidence
boundary. Revision changes require a new capability manifest, patch rebase,
generated-code review, and complete verification.

## Licensing

This repository is Apache-2.0. Yew is MIT OR Apache-2.0. Lynx, PrimJS,
Habitat, FlatBuffers, Dioxus, and `@lynx-js/tasm` retain their upstream terms.
See [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
