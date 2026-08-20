# Stock Lynx MTS/Fiber broker

This directory contains the ordinary LepusNG adapter between the synchronous
`LynxElementBridge` native module and Lynx's public Fiber Element API. The shell
is backend-neutral and uses the public module name shared by both backends.

- `src/wire.mts` verifies and encodes FlatBuffers v2 `LEB2` envelopes.
- `src/broker-core.js` validates batches and owns every `ElementRef`, node
  relation, and event callback.
- `src/lynx-fiber-host.mts` maps typed commands to public Fiber globals.
- `src/shell-core.js` owns render, reload, removal, SSR rejection, and lifetime
  teardown and binds the fixed `LynxElementBridge` module name.
- `PROTOCOL.md` defines the exact native wire and lifecycle contract.

## Build And Test

Node.js 22.18.0 is pinned at the repository root. From this directory:

```bash
npm ci
npm run build
npm run build:wasm
npm test
```

The build emits ignored shell, encoder-input, and template bundle artifacts
under `dist/`. Both codec paths decode the result and require `context-type=1`
and a LepusNG binary.

## Stock API Boundary

The broker uses the public typed globals declared by the pinned revision's
`@lynx-js/type-element-api` source. Stock LepusNG registers those Fiber globals
and the synchronous `lynx.module()` proxy. `lynx.module()` is present in the
pinned implementation but absent from that revision's stable public `Lynx`
TypeScript interface, so it is a revision-pinned integration surface.

Android `LynxMethodWrapper` supports `byte[]`, but ordinary LepusNG did not
expose readable ByteArray contents. `patches/lynx` adds a minimal read-only
`length` and numeric-index view so the FlatBuffers reader can consume native
buffers without Base64 or string conversion.

## Protocol And Lifecycle

- IDs are opaque unsigned 32-bit numbers, not strings.
- Commands, results, and events use distinct `LEB2` channels.
- Host query values return through `completeBatch`; callbacks send complete
  `EventMessage` buffers through `dispatchEvent`.
- The generated dispatcher covers all 107 declarations in the pinned Element
  API package. The capability manifest reports unavailable optional operations
  as `UNSUPPORTED`.
- Complete batches are validated before Fiber mutation. Calls are synchronous
  on the session's mounting thread and reentrancy is rejected.
- One shell has one active broker, and one module instance owns at most one live
  Rust session.
- Cached roots and SSR hydration are rejected. Reload and lifetime destruction
  explicitly tear down listeners, tree ownership, and native session state.

Tests use mock Fiber globals and a mock native module. They verify the binary
wire, typed dispatch, result completion, event envelopes, lifecycle, and public
module name, but do not establish device behavior.
