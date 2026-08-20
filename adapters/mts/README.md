# Stock Lynx MTS/Fiber broker

This directory contains the ordinary LepusNG adapter between the synchronous
`YewLynx` native module and stock Lynx's public Fiber Element APIs.

- `src/broker-core.js` validates protocol v1 and owns every `ElementRef`, node
  relation, and event callback.
- `src/lynx-fiber-host.mts` maps validated operations to public typed Fiber
  globals.
- `src/shell-core.js` owns render, reload, removal, SSR rejection, and lifetime
  teardown.
- `template/shell.mts` installs the shell in an ordinary Lynx template.
- `template/template.config.json` selects Fiber and `contextType: 1` LepusNG.
- `PROTOCOL.md` defines the exact native wire contract.

## Build and test

Node.js 22.18.0 is pinned at the repository root. From this directory:

```bash
npm ci
npm run build
npm run build:wasm
npm test
```

The exact template-build dependencies are `esbuild` 0.25.9 and
`@lynx-js/tasm` 0.0.51. The build emits ignored artifacts:

```text
dist/shell.js
dist/template-input.json
dist/yew-lynx-counter.lynx.bundle
```

The ordinary build uses the packaged N-API codec where supported and otherwise
falls back to WebAssembly. `build:wasm` forces that fallback in CI. After
encoding, each build decodes the bundle and fails unless
`context-type === 1` and `is-lepusng-binary === true`. This verifies an
ordinary LepusNG template artifact, not a native-runtime descriptor.

## Serve the bundle locally

To expose the generated template bundle from a local HTTP service, run:

```bash
npm run serve
```

`serve` rebuilds the template with the pinned Lynx template encoder and serves
`dist/yew-lynx-counter.lynx.bundle` at:

```text
http://127.0.0.1:4173/yew-lynx-counter.lynx.bundle
```

It also prints a terminal QR code for the served template URL. To suppress QR
output, use `npm run serve -- --no-qr` or set `NO_QR=1`.

If the bundle has already been built and only needs to be hosted, use:

```bash
npm run serve:bundle
```

For a physical device on the same network, bind the service to all interfaces:

```bash
npm run serve -- --host 0.0.0.0
```

This prints QR codes for both the loopback URL and each detected IPv4 LAN URL.

## Stock API boundary

The broker uses the public typed globals declared by the pinned revision's
[`@lynx-js/type-element-api` source](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/js_libraries/type-element-api/types/element-api.d.ts).
Stock LepusNG registers those Fiber functions and `lynx.module()` in
[`renderer_ng.cc`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/runtime/lepusng/bindings/renderer_ng.cc),
and module lookup returns the synchronous proxy implemented by
[`lynx_lepus_module_manager.cc`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/runtime/lepus/bindings/modules/lynx_lepus_module_manager.cc).

`lynx.module()` is not declared by the same revision's stable public
[`Lynx` TypeScript interface](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/js_libraries/types/types/main-thread/lynx.d.ts).
The shell therefore treats it as revision-pinned stock behavior, not a stable
declared TypeScript API.

## Protocol and lifecycle

- IDs are positive safe integers no greater than `Number.MAX_SAFE_INTEGER`.
  Root and listener IDs are passed to the Java module as decimal strings.
- The broker rejects unknown fields and invalid ownership before Fiber
  mutation. Every success, including a no-op, has one final flush.
- Initial mount validates but skips its protocol flush because Lynx's enclosing
  render pipeline flushes after `__RenderPage` returns. Event, update remount,
  and destroy paths flush normally.
- Protocol v1 supports `tap` only. Calls and event updates are synchronous and
  must remain on the session's mounting thread; nested operations and
  reentrancy are rejected.
- Raw text may only be inserted directly beneath a `<text>` element.
- One shell has one active broker and one module instance has at most one live
  Rust session.
- Cached `initPage` roots, nonempty cache data, and SSR hydration are rejected
  because the broker cannot adopt `ElementRef` values it did not create.
- Reload destroys then remounts. Component removal destroys the broker and a
  later render may mount a fresh session. `__DestroyLifetime` performs explicit
  teardown.
- A host mutation failure poisons the broker because Fiber mutations are not
  transactional; destroy remains a best-effort cleanup path.

The tests use mock Fiber globals and a mock native module. They do not load the
bundle into a stock Lynx runtime or establish renderer/device behavior.
