# Framework adapter authoring

This guide defines the boundary for adding a Rust UI framework to Lynx Element
Bridge. Yew and Dioxus are the reference adapters.

## Adapter Boundary

An adapter translates one synchronous framework render turn into
`lynx_element_bridge_core::CommandBatch`. The batch is an in-memory ordered
`Vec<Command>`; it is not a serialization format.

The core owns:

- Node, listener, and callback IDs.
- Owner-thread, tree ownership, and exact listener validation.
- Batch sequencing and final commit boundaries.
- Opaque event content type and payload bytes.
- Deterministic subtree and listener teardown.

The adapter owns:

- Mapping framework-local node IDs to bridge `NodeId` values.
- Lowering framework templates, placeholders, stacks, or VNodes.
- Holding callbacks while their bridge listener is live.
- Translating validated events into framework-specific event objects.
- Recording the first framework error and discarding pending mutations.

Host element handles never enter adapter state. `NativeHost` maps bridge IDs to
opaque handles only while applying a batch through `LynxNativeRendererApiV1`.

## Session And Rendering

Create a session with a caller-owned root. The enclosing native registry entry or
WASM runtime instance provides lifecycle isolation; messages do not carry that
outer handle:

```rust
let session = Session::create(root)?;
```

Call the corresponding `Session` mutation method in framework order and finish
the render turn with `take_batch()`. Do not manufacture sequence values or
commit flags in adapter code.

Yew implements `NativeRendererBackend` in `adapters/yew/src/lib.rs`. Dioxus
implements `WriteMutations` in `adapters/dioxus/src/lib.rs`; Dioxus `Template`,
template paths, placeholder replacement, and stack behavior remain local to
that adapter.

Framework application lifecycle belongs in `runtimes/yew` and
`runtimes/dioxus`, not in an example or adapter. The `lynx` facade exposes the
runtime seam as `lynx::yew::launch!` and `lynx::dioxus::launch!`; these macros
emit Native or WASM entrypoints in the final application crate.

## Events

Register callbacks under core `CallbackId` and `ListenerId` values. The native
host callback validates renderer, listener, callback, and event name before
building `EventMessage`. Content type and payload bytes remain opaque.

Callbacks and any resulting render run synchronously on the session owner
thread. Do not schedule across threads or permit reentrant entry. Recoverable
identity errors reject only that callback; a framework panic or native mutation
failure poisons the session.

## Destroy

Destroy framework state first when it emits normal renderer cleanup, then call
the adapter/core destroy path for any remaining listeners and descendants. The
final batch leaves the caller-owned root intact. Clear framework-local node and
callback maps.

Implement `BridgeBackend::abandon` when the framework owns resources that must
be disconnected without producing host mutations. Abandon is not an ordinary
unmount path.

## Required Tests

Add the adapter to `crates/adapter-conformance`. The shared public seam must
prove the same observable behavior for:

1. Initial mount.
2. One event with opaque payload bytes.
3. One framework update.
4. Explicit destroy and listener cleanup.

Also add a real framework application and include the shared native lifecycle
tests. Those tests must exercise native mount, event callback, update,
normal destroy, abandon, wrong-thread, reentry/busy, poisoning, panic
containment, and stale-session behavior.

Run `./scripts/verify.sh` before publishing an adapter package.
