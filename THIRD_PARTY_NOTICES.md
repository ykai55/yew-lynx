# Third-party notices

This repository is licensed under Apache-2.0. The following projects are
patched, downloaded for verification, or cited as public-source references.

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

`patches/yew/` contains modifications intended for this exact Yew revision.
The bootstrap script downloads Yew into ignored local state; the upstream
checkout is not vendored in this repository. The upstream checkout retains both
license files. This repository distributes its modifications under
Apache-2.0, without changing the license terms applicable to upstream Yew code.

## Lynx

- Project: Lynx
- Upstream: <https://github.com/lynx-family/lynx>
- Audited reference revision: `0df14207cebb060f1bed8de12b64a1119dee8f06`
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

Lynx source and binaries are not copied, patched, downloaded by the bootstrap
script, or included as a submodule. `docs/oss-lynx-gap.md` cites public Lynx
source solely to document the compatibility gap at the audited revision.

The names Yew and Lynx identify their respective upstream projects. No
endorsement, official support, or affiliation is implied.
