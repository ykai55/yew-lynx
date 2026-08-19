# Third-party notices

This repository is licensed under Apache-2.0. The following projects are
patched, downloaded for development and verification, or cited as pinned
public-source references.

## Yew

- Project: Yew
- Upstream: <https://github.com/yewstack/yew>
- Patch base: `0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3`
- Copyright notice from the upstream MIT license: Copyright (c) 2017 Denis
  Kolodin
- License: MIT OR Apache-2.0, at the user's option
- License files:
  [MIT](https://github.com/yewstack/yew/blob/0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3/LICENSE-MIT) and
  [Apache-2.0](https://github.com/yewstack/yew/blob/0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3/LICENSE-APACHE)

`patches/yew/` contains modifications for this exact revision. The bootstrap
script downloads Yew into ignored local state; the upstream checkout is not
vendored. It retains both upstream license files. This repository distributes
its modifications under Apache-2.0 without changing upstream Yew's terms.

## Lynx and @lynx-js/tasm

- Project: Lynx
- Upstream: <https://github.com/lynx-family/lynx>
- Audited source revision: `0df14207cebb060f1bed8de12b64a1119dee8f06`
- Development package: [`@lynx-js/tasm` 0.0.51](https://www.npmjs.com/package/@lynx-js/tasm/v/0.0.51)
- License: Apache-2.0
- License file:
  <https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/LICENSE>

The upstream NOTICE at the audited revision states:

```text
Lynx Project
Copyright (c) 2018-2024 ByteDance Inc.
Copyright (c) 2024 TikTok Inc.
All rights reserved.
```

The exact Lynx source revision is included as the `third_party/lynx` submodule
and built without source patches. Its six Android modules are published only to
an ignored local Maven repository. `npm ci` also downloads the published
`@lynx-js/tasm` package for development and verification. That package contains
Apache-2.0 Lynx code and platform native binary modules used to encode and
decode the generated template bundle.

## PrimJS

- Project: PrimJS
- Upstream: <https://github.com/lynx-family/primjs>
- Runtime artifacts: `primjs` and `primjsWasm`
  `4.2.0-alpha.0-20260731.091808-1`
- License: Apache-2.0
- License file: <https://github.com/lynx-family/primjs/blob/develop/LICENSE>

The pinned Lynx Android build requires these runtime AARs. Their timestamped
identities and AAR/POM SHA-256 values are recorded in `android/primjs.lock`;
`scripts/prepare-primjs.sh` verifies those bytes before exposing an isolated
local Maven repository. The resulting arm64 runtime libraries are packaged in
the example APK.

## Habitat

- Project: Habitat
- Upstream: <https://github.com/lynx-family/habitat>
- Build executable: `hab.pex` 0.3.149
- License: Apache-2.0
- License file: <https://github.com/lynx-family/habitat/blob/0.3.149/LICENSE>

Habitat synchronizes the pinned Lynx source dependencies during the Android
build. `android/hab.lock` records the executable SHA-256, and
`scripts/prepare-hab.sh` verifies it before execution. Habitat is a build tool
and is not packaged in the APK.

## esbuild

- Project: esbuild
- Upstream: <https://github.com/evanw/esbuild>
- Development package: [`esbuild` 0.25.9](https://www.npmjs.com/package/esbuild/v/0.25.9)
- Copyright: Copyright (c) 2020 Evan Wallace
- License: MIT
- License file:
  <https://github.com/evanw/esbuild/blob/v0.25.9/LICENSE.md>

`npm ci` downloads esbuild and the matching optional platform binary package.
They are development dependencies used to bundle the MTS shell before template
encoding and are not vendored as tracked repository files.

The names Yew, Lynx, and esbuild identify their respective upstream projects.
No endorsement, official support, or affiliation is implied.
