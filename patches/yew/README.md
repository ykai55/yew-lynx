# Yew native_renderer patch series

This host-independent patch adds the synchronous renderer contract used by the
Yew backend. It does not depend on Lynx or expose a concrete C ABI; this
repository composes it with the core `Session`, in-memory `CommandBatch`, Rust
`NativeHost`, and Lynx's versioned native renderer function table.

## Base Revision

- Upstream: <https://github.com/yewstack/yew>
- Commit: `0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3`
- Required Rust version: 1.85

The patch adds a `native_renderer` feature to `yew` and `yew-macro`. It supports
native targets and `wasm32-wasip1`, cannot be combined with `csr` or
`hydration`, and does not select Yew's browser DOM renderer. Browser-only Gloo,
panic-hook, and `wasm-bindgen-futures` dependencies remain tied to CSR or
hydration rather than the WASI guest.

## API

Enabling `native_renderer` exposes:

- `NativeRendererBackend`, the synchronous host mutation trait.
- `NativeNode` and `NativeListener`, opaque integer identities.
- `NativeEvent`, currently carrying the validated event name.
- `NativeRenderer`, which mounts one component into a caller-owned root.
- `NativeAppHandle`, which sends messages, explicitly destroys the application,
  or abandons Rust state without touching an unreachable host.
- `NativeRendererBusy`, returned when destroy is attempted from an active Yew
  scheduler callback and the caller must retain and retry the handle.

Renderer, scheduler, lifecycle, backend, and event callback work stays on the
renderer owner thread. `NativeRenderer::render()` rejects entry while Yew's
scheduler is already executing lifecycle, update, or event work.

## Ownership Contract

- Newly created nodes are detached.
- `insert_before` attaches a detached node to one direct parent.
- `remove` detaches without destroying.
- `destroy_node` receives a detached, childless node after listener and subtree
  teardown.
- The supplied root stays caller-owned.
- Listener removal synchronously disconnects and releases its callback.
- `flush` commits the current render turn.

The renderer validates a complete VNode tree before mutation. A changed valid
tree is attached before the previous tree is detached; equal VNodes are
retained. This patch does not implement fine-grained in-place text updates.

## Supported Surface

| Surface | Current behavior |
| --- | --- |
| Top-level component | Messages, hooks, synchronous `rendered`, explicit destroy |
| `VNode::VText` | Created through `NativeRendererBackend::create_text` |
| `VNode::VTag` | Authored tags, attributes, children, `value`, `checked` |
| `VNode::VList` | Source-order children |
| Events | `ontap` with a typed `NativeEvent` callback |
| Nested components, refs, portals, suspense, raw HTML, keys | Rejected or unsupported as documented by patch tests |

The bridge retains native event payload bytes in `EventMessage`; the current Yew
patch intentionally exposes only the validated event name to component code.

## Panic And Cleanup

With `panic = "unwind"`, initial render and destroy cleanup guards remove tested
tree/listener state before resuming a component panic. Backend methods must not
panic because a second panic during unwind can abort. With `panic = "abort"`,
cleanup guards cannot run.

Call `NativeAppHandle::destroy(&mut self)` explicitly and handle its result.
`NativeAppHandle::abandon(&mut self)` only breaks renderer-owned Rust state and
is reserved for emergency teardown.

## Applying And Verification

Use the repository bootstrap for the exact base revision:

```bash
./scripts/bootstrap-yew.sh
```

The patch updates Yew's lockfile with the base manifest's existing `matchit
0.9.2` dependency so Cargo 1.85 accepts locked commands. Focused checks are:

```bash
cargo check --locked -p yew --features native_renderer
cargo check --locked --target wasm32-wasip1 -p yew --features native_renderer
cargo test --locked -p yew --lib --features native_renderer native_renderer::tests
cargo test --locked -p yew-macro --features native_renderer --test html_macro_test html_macro -- --exact
cargo test --locked -p yew-macro --test html_macro_test html_macro -- --exact
```

`./scripts/verify.sh` runs these with workspace conformance, public native
lifecycle, Android JNI, real staticlib symbol, and Lynx patch/header checks.

The composed Yew backend passed final patch-0015 binary-native device acceptance
on 2026-08-22 on an anonymous Xiaomi Redmi K60 Pro physical device running
Android 13/API 33, arm64-v8a. Fresh launch, tap, recreation,
force-stop/reopen, and three cycles passed, with every functional result flag
true. The run recorded
`onCreate=9`, `onDestroy=4`, `diagnostics=9`, nine Yew backend markers,
`renderer_mode=native`, `bts_runtime=false`, `mts_context=false`, and
`template=false`, with zero crash markers and `proc_maps_checked=true`. Process
maps confirmed all five required libraries after fresh launch and interaction,
an empty forbidden set, and false Quick/NAPI/Wasm/V8 mapping flags. The tested
APK SHA-256 is
`121c20a6bc82d1570eb24cfb37c84a5d82c414bc1b176c4b40ac8294fe45903d`; evidence
is `.deps/android/device-acceptance-binary-native-yew-20260822-0015-final`.

The app uses the opt-in
`org.lynxsdk.lynx:lynx-native-renderer:0.0.1-0df14207` /
`liblynx_native_renderer.so` product. It does not depend on the separately
published stock `org.lynxsdk.lynx:lynx:0.0.1-0df14207` / `liblynx.so` product or
its JavaScript runtime dependencies. Rendering remains direct through the C ABI,
without MTS or template transport.
