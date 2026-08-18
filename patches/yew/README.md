# Yew patch series

This directory contains a public, host-independent patch for Yew's experimental
Clay renderer. It keeps the tested renderer and reconciliation surface small and
does not add a concrete host backend.

## Base revision

- Upstream: <https://github.com/yewstack/yew>
- Commit: `0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3`
- Required Rust version: 1.85, as declared by that Yew revision

Apply the patches in the order listed by `series`. They are not intended for a
different Yew revision without rebasing and rerunning the checks below.

## Features and API

The patch adds a `clay` feature to `yew` and `yew-macro`. It is native-only:
building `yew` with `clay` for `wasm32`, or combining `clay` with `csr` or
`hydration`, produces a compile error. `clay` may be combined with `ssr` on a
native target.

Enabling `clay` exposes:

- `ClayBackend`, the host mutation interface
- `ClayNode` and `ClayListener`, opaque host-owned handles
- `ClayEvent`, which currently carries only an event name
- `ClayRenderer` and `ClayAppHandle`, which mount and destroy one top-level Yew
  component

The renderer uses Yew's existing component scheduler and lifecycle. Mounting,
component messages, hook state updates, reconciliation, `rendered`, destruction,
and backend flushes run synchronously on the renderer's owner thread.

A consumer must implement `ClayBackend` using public host APIs. The backend owns
node creation, insertion, removal, attributes, event registrations, destruction,
and flushing. Its ownership contract is:

- A newly created node is detached.
- `insert_before` gives a detached node one direct parent.
- `remove` detaches a direct child without destroying it.
- Before `destroy_node`, the renderer removes the node from its direct parent,
  unregisters its listeners, recursively removes and destroys its direct
  children, and leaves the node detached and childless.
- The root passed to `ClayRenderer` is caller-owned and is never destroyed.
- `remove_event_listener` must synchronously disconnect and release its callback;
  the callback must never run again after removal.
- Every backend method and registered callback must run on the renderer's owner
  thread. Backend methods must not panic, particularly during unwind cleanup.

## Supported surface

The current prototype intentionally supports only the surface exercised by its
tests:

| Surface | Support |
| --- | --- |
| Top-level Yew component | Supported, including messages, hooks, synchronous `rendered`, and destroy lifecycle |
| `VNode::VText` | Supported through `ClayBackend::create_text` |
| `VNode::VTag` | Supported for authored tags, attributes, children, `value`, and `checked`; `checked=false` clears the host attribute |
| `VNode::VList` | Supported with source-order children |
| Nested `VNode::VComp` | Not supported; validation panics before host mutation |
| `VNode::VRef` | Not supported because it represents a browser DOM node |
| `VNode::VPortal` | Not supported |
| `VNode::VSuspense` | Not supported |
| `VNode::VRaw` | Not supported because raw HTML has no host-independent meaning |
| `NodeRef` bindings | Not supported; an explicit binding is rejected before host mutation |
| Keys | Not supported; any keyed VNode or key change is rejected before host mutation |

The renderer validates a complete VNode tree before mutating the host. Changed,
supported trees are attached before the previous tree is detached, while equal
VNodes are retained. The prototype does not perform fine-grained text or
attribute updates.

The event surface is limited to `ontap`:

- With `clay` enabled, `ontap={callback}` is a typed listener receiving
  `ClayEvent`.
- `ClayEvent::name()` reports the backend event name; no payload, target, or
  propagation data is exposed.
- A rendered listener other than `ontap` is rejected with a panic.
- Without `clay`, `ontap="..."` remains an ordinary element attribute.

## Panic behavior

Unsupported VNode output, explicit `NodeRef` bindings, keys, and unsupported
listeners currently panic rather than returning `Result`. Validation happens
before host mutation, so an unsupported update leaves the previous Clay tree
attached.

With `panic = "unwind"`, `ClayRenderer::render` tears down an initial tree if
mount, render, or `rendered` unwinds. A cleanup guard also removes all Clay nodes
and listeners if `Component::destroy` unwinds, then the original panic resumes.
These guarantees require the documented no-panic backend contract.

With `panic = "abort"`, the process terminates and no rollback or teardown is
promised. Panics from arbitrary component update or lifecycle code are not a
transactional rollback mechanism. Call `ClayAppHandle::destroy`; dropping the
handle alone does not schedule host teardown.

## Applying

Start from a clean checkout of the exact base revision:

```bash
git clone https://github.com/yewstack/yew.git
git -C yew checkout --detach 0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3
git -C yew am /absolute/path/to/patches/yew/*.patch
```

To apply `series` explicitly, run this from the `yew-lynx` repository root after
setting `YEW_DIR` to the clean Yew checkout:

```bash
while IFS= read -r patch; do
  git -C "$YEW_DIR" am "$PWD/patches/yew/$patch"
done < patches/yew/series
```

## Lockfile

The patch updates `Cargo.lock` with the existing `matchit 0.9.2` dependency of
`yew-router-macro`. The base manifest already declares that dependency, but the
base lockfile omits it; Cargo 1.85 revalidates the workspace after the patched
manifests change and otherwise rejects every `--locked` command.

## Verification

Run these commands from the patched Yew checkout:

```bash
rustc --version
git diff --check 0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3..HEAD
cargo check --locked -p yew --features clay
cargo check --locked -p yew --features clay,ssr
cargo test --locked -p yew --lib --features clay
cargo test --locked -p yew-macro --features clay --test html_macro_test html_macro -- --exact
cargo test --locked -p yew-macro --test html_macro_test html_macro -- --exact
```

The following commands are expected to fail with the explicit feature/target
diagnostics:

```bash
cargo check --locked -p yew --features clay,csr
cargo check --locked -p yew --target wasm32-unknown-unknown --features clay
```
