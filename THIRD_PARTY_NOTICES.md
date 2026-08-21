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

The exact source is the `third_party/lynx` gitlink. `patches/lynx/0002-0009`
add the native renderer function table, Android host registration, lifecycle,
diagnostics, event delivery, and tests. Lynx Android modules are published only
to an ignored local Maven repository.

## Dioxus

- Upstream: <https://github.com/DioxusLabs/dioxus>
- Runtime crate: `dioxus-core` 0.7.10
- License: MIT OR Apache-2.0
- License files: <https://github.com/DioxusLabs/dioxus/tree/v0.7.10/LICENSES>

The Dioxus adapter uses the renderer-neutral core API and retains Dioxus
framework `Template` types locally.

## PrimJS

- Upstream: <https://github.com/lynx-family/primjs>
- Runtime artifacts: `primjs` and `primjsWasm`
  `4.2.0-alpha.0-20260731.091808-1`
- License: Apache-2.0
- License file: <https://github.com/lynx-family/primjs/blob/develop/LICENSE>

The pinned Lynx Android source build requires these AARs. Their identities and
SHA-256 values are recorded in `android/primjs.lock` and verified by
`scripts/prepare-primjs.sh`. The stock Lynx AAR still packages and loads Quick,
PrimJS, and NAPI. Runtime-native renderer diagnostics establish only the active
path; binary-native packaging remains a blocked follow-up milestone, and
complete JS-engine removal is not claimed.

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
