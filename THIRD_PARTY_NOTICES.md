# Third-party notices

This repository is Apache-2.0. The projects below are patched, linked,
downloaded for build/verification, or cited as pinned public-source references.
Each upstream project retains its own terms.

## Yew

- Upstream: <https://github.com/yewstack/yew>
- Patch base: `0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3`
- License: MIT OR Apache-2.0
- License files:
  <https://github.com/yewstack/yew/tree/0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3>

`patches/yew/` modifies this exact revision. `scripts/bootstrap-yew.sh`
materializes an ignored checkout retaining upstream license files. This
repository's modifications are Apache-2.0 and do not change Yew's terms.

## Lynx

- Upstream: <https://github.com/lynx-family/lynx>
- Audited source revision: `0df14207cebb060f1bed8de12b64a1119dee8f06`
- License: Apache-2.0
- License file:
  <https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/LICENSE>

The upstream NOTICE at this revision states:

```text
Lynx Project
Copyright (c) 2018-2024 ByteDance Inc.
Copyright (c) 2024 TikTok Inc.
All rights reserved.
```

The exact source is the `third_party/lynx` gitlink. The 15-patch `patches/lynx`
series (`0002-0016`) adds the native renderer function table, Android host
registration, lifecycle, diagnostics, event delivery, native-only Android
product, and tests. The native-only JNI registration filter is applied
reproducibly to the Lynx-pinned tools_shared revision
`ff47fee7d41ee3e8e8561041b1ce2c8b50e923ea` by `patches/lynx-tools-shared`. Lynx
Android modules are published only to an ignored local Maven repository.

The build publishes the preserved stock coordinate
`org.lynxsdk.lynx:lynx:0.0.1-0df14207` with `liblynx.so`, and the separate
opt-in coordinate
`org.lynxsdk.lynx:lynx-native-renderer:0.0.1-0df14207` with
`liblynx_native_renderer.so`. The stock product's behavior is unchanged. The
native product and example app do not depend on stock `lynx`, LynxJSSDK, or the
JavaScript runtime artifacts.

## Dioxus

- Upstream: <https://github.com/DioxusLabs/dioxus>
- Direct crates: `dioxus-core`, `dioxus-core-macro`, and `dioxus-signals` 0.7.10
- License: MIT OR Apache-2.0
- License files: <https://github.com/DioxusLabs/dioxus/tree/v0.7.10/LICENSES>

The Dioxus adapter uses the renderer-neutral core API and exposes a Lynx-native
RSX vocabulary. Dioxus framework `Template` values remain in-memory VDOM data.

## PrimJS

- Upstream: <https://github.com/lynx-family/primjs>
- Runtime artifacts: `primjs` and `primjsWasm`
  `4.2.0-alpha.0-20260731.091808-1`
- License: Apache-2.0
- License file: <https://github.com/lynx-family/primjs/blob/develop/LICENSE>

The preserved stock Lynx Android source build requires these AARs. Their
identities and SHA-256 values are recorded in `android/primjs.lock` and verified
by `scripts/prepare-primjs.sh`, so the stock artifact can be built and tested
reproducibly. This preparation and attribution remain required for that stock
artifact. PrimJS is not a dependency of the native renderer product or example
app; their dependency graphs, APK, and runtime process maps exclude PrimJS,
Quick, NAPI, Wasm, and V8.

## Habitat

- Upstream: <https://github.com/lynx-family/habitat>
- Build executable: `hab.pex` 0.3.149
- License: Apache-2.0
- License file: <https://github.com/lynx-family/habitat/blob/0.3.149/LICENSE>

Habitat synchronizes pinned Lynx source dependencies during Android builds.
`android/hab.lock` records its SHA-256 and `scripts/prepare-hab.sh` verifies it.
Habitat is not packaged in the APK.

The names Yew, Lynx, Dioxus, PrimJS, and Habitat identify their respective
upstream projects. No endorsement, official support, or affiliation is implied.
