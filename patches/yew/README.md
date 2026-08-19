# Yew native_renderer patch series

This directory contains a public, host-independent patch adding an experimental
native renderer to Yew. It provides the narrow renderer contract used by this
repository and does not depend on Lynx or include a concrete host backend.

## Base revision

- Upstream: <https://github.com/yewstack/yew>
- Commit: `0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3`
- Required Rust version: 1.85

Apply patches in the order listed by `series`. Other Yew revisions require a
rebase and complete reverification.

## Feature and API

The patch adds a `native_renderer` feature to `yew` and `yew-macro`. It is
native-target only, cannot be combined with `csr` or `hydration`, and may be
combined with `ssr` on a native target.

Enabling `native_renderer` exposes:

- `NativeRendererBackend`, the synchronous host mutation trait
- `NativeNode` and `NativeListener`, opaque host-owned integer handles
- `NativeEvent`, currently carrying only the event name
- `NativeRenderer`, which mounts one top-level component into a caller-owned
  root
- `NativeAppHandle`, which sends messages through its dereferenced `Scope`,
  explicitly destroys the mounted application through a mutable handle, or
  abandons Rust state without touching an already-unreachable host
- `NativeRendererBusy`, returned by `NativeAppHandle::destroy(&mut self)` when
  called from an active Yew scheduler callback so the caller retains the handle
  and can retry

All renderer, scheduler, lifecycle, backend, and event callback work runs
synchronously on the renderer owner thread. A backend must not move callbacks
to another thread or panic.

`NativeRenderer::render()` rejects calls made while Yew's scheduler is already
executing a component lifecycle, update, or event callback. Destruction in that
state returns `NativeRendererBusy` without consuming or queueing the handle.
Without these guards, create and destroy could be queued in the wrong priority
order or stranded by a later panic.

The backend ownership contract is:

- Newly created nodes are detached.
- `insert_before` gives a detached node one direct parent.
- `remove` detaches a direct child without destroying it.
- Before `destroy_node`, the renderer removes the node from its direct parent,
  removes listeners, recursively tears down direct children, and leaves the
  node detached and childless.
- The root supplied to `NativeRenderer` remains caller-owned and is never
  destroyed.
- `remove_event_listener` synchronously disconnects and releases the callback;
  it must never run again.
- `flush` commits pending host mutations for the supplied root.

## Supported surface

| Surface | Current behavior |
| --- | --- |
| Top-level component | Messages, hooks, synchronous `rendered`, and explicit destroy lifecycle |
| `VNode::VText` | Created through `NativeRendererBackend::create_text` |
| `VNode::VTag` | Authored tags, attributes, children, `value`, and `checked`; initial `checked=false` clears the host attribute |
| `VNode::VList` | Source-order children |
| Nested `VNode::VComp` | Rejected before host mutation |
| `VNode::VRef`, portals, suspense, and raw HTML | Not supported |
| Explicit `NodeRef` bindings | Rejected before host mutation |
| Keys | Not supported; a keyed VNode or key introduced by an update is rejected before host mutation |
| Events | `ontap` only, with a typed `NativeEvent` callback |

The renderer validates a complete VNode tree before mutation. A changed valid
tree is attached before the previous tree is detached; equal VNodes are
retained. The prototype does not perform fine-grained in-place text or
attribute updates.

Without `native_renderer`, `ontap="..."` remains an ordinary authored
attribute. With the feature enabled, `ontap={callback}` is a typed listener.
`NativeEvent::name()` exposes only `"tap"`; payload, target, propagation, and
other events are outside the current contract.

## Panic and cleanup behavior

Unsupported output currently panics instead of returning `Result`. Validation
keeps selected invalid output from mutating the host.

With `panic = "unwind"`, `NativeRenderer::render` tears down an initial tree if
mount, render, or `rendered` unwinds. Destroy cleanup guards remove the tested
tree and listeners if `Component::destroy` unwinds, then resume the original
panic. Backend operations must remain non-panicking because a second panic
during unwind can abort.

With `panic = "abort"`, cleanup guards cannot run. General component
create/update/lifecycle panics are not a transactional recovery mechanism.
Call `NativeAppHandle::destroy(&mut self)` explicitly and handle its `Result`;
dropping the handle does not schedule host teardown.
`NativeAppHandle::abandon(&mut self)` only breaks renderer-owned Rust state and
never mutates the host; it is reserved for emergency teardown after that host
or owner thread is already unavailable.

## Applying

From a clean checkout of the exact base revision:

```bash
git clone https://github.com/yewstack/yew.git
git -C yew checkout --detach 0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3
export YEW_DIR="$PWD/yew"
while IFS= read -r patch; do
  git -C "$YEW_DIR" am "$PWD/patches/yew/$patch"
done < patches/yew/series
```

The loop is intended to run from the `yew-lynx` repository root, with
`YEW_DIR` pointing to the clean Yew checkout. The repository bootstrap performs
the same pinned, identity-checked operation automatically:

```bash
./scripts/bootstrap-yew.sh
```

The patch updates Yew's `Cargo.lock` with the base manifest's existing
`matchit 0.9.2` dependency so Cargo 1.85 accepts locked commands after the
manifest changes.

## Verification

Run from the patched Yew checkout:

```bash
rustc --version
git diff --check 0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3..HEAD
cargo check --locked -p yew --features native_renderer
cargo check --locked -p yew --features native_renderer,ssr
cargo test --locked -p yew --lib --features native_renderer native_renderer::tests
cargo test --locked -p yew-macro --features native_renderer --test html_macro_test html_macro -- --exact
cargo test --locked -p yew-macro --test html_macro_test html_macro -- --exact
```

These commands are expected to fail with the patch's explicit diagnostics:

```bash
cargo check --locked -p yew --features native_renderer,csr
cargo check --locked -p yew --target wasm32-unknown-unknown --features native_renderer
```

From the `yew-lynx` repository root, `./scripts/verify.sh` runs the focused
patched-Yew checks together with the Rust, template, broker, and Android mock
checks.
