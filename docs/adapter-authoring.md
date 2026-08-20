# Framework adapter authoring

This guide describes the boundary a third UI framework must implement to use
Lynx Element Bridge. The Yew and Dioxus adapters are the reference examples.

## Adapter Boundary

An adapter translates framework renderer operations into
`lynx_element_bridge_core::CommandBatch`. It does not own host ElementRefs,
allocate cross-session IDs, execute Lynx globals, or define wire encoding.

The core owns:

- Session, node, listener, callback, and result-slot identity.
- Required and optional capability negotiation.
- Parent/child and listener ownership validation.
- Owner-thread checks, batch sequence, and final commit.
- Framework-neutral Result and Event channel values.
- Deterministic subtree and listener teardown.

The adapter owns:

- Mapping framework-local node IDs to bridge `NodeId` values.
- Lowering framework templates, placeholders, stacks, or VNodes.
- Holding framework callbacks while their bridge listener is live.
- Translating opaque Event payloads into framework-specific event objects.
- Reporting unsupported framework output before a batch is submitted.

## Session Setup

Construct the core session with a nonzero `SessionId`, caller-owned root
`NodeId`, and explicit capability requests. Mutation capabilities needed for a
valid render should be required. Optional enhancements should be optional and
must handle item-level `Status::Unsupported` results.

```rust
let requests = [
    CapabilityRequest::required("create_element"),
    CapabilityRequest::required("append_element"),
    CapabilityRequest::optional("set_static_style"),
];
let (session, negotiated) = Session::create(session_id, root, &requests)?;
```

Do not derive bridge IDs from pointer values or reusable framework IDs. Keep a
framework-local map and let `Session` allocate bridge nodes and listeners.

## Rendering

Call the corresponding `Session` method for each framework mutation in source
order. End one synchronous rendering turn with `take_batch()`. Never manufacture
`CommandBatch.sequence` or set `final_commit` in adapter code.

Yew implements `NativeRendererBackend` in `adapters/yew/src/lib.rs`. Dioxus
implements `WriteMutations` in `adapters/dioxus/src/lib.rs`; template paths,
placeholder replacement, and stack operations remain Dioxus-specific there.

If framework APIs report errors through callbacks rather than `Result`, record
the first adapter error, discard pending core commands, and reject the batch.
Do not expose a partially validated framework render.

## Results

Allocate a `ResultSlot` before emitting a query. The host responds with a
`ResponseBatch` carrying per-item status and one of the framework-neutral result
values: element ID(s), string(s), boolean, number, or opaque payload.

Element-returning commands use preallocated bridge node IDs. A host ElementRef
never crosses the wire. The MTS broker stores the ElementRef under the reserved
ID and returns that ID in the Result item.

Treat a missing required capability as a session-creation failure. Treat an
optional gap as a normal `UNSUPPORTED` item; do not poison the adapter or assume
the rest of the batch was rolled back.

## Events

Register framework callbacks under core `CallbackId` and `ListenerId` values.
The host sends `EventMessage { content_type, payload }`; the bridge does not
parse the bytes. Validate session, listener, callback, and event name before
entering framework code.

Callbacks and all resulting renders stay synchronous on the owner thread.
Reject reentrant or cross-thread entry rather than scheduling implicitly.

## Destroy

Destroy framework state first when it emits normal renderer cleanup, then call
the adapter/core destroy path to release anything still owned by the session.
The resulting final batch must remove listeners and descendants while leaving
the caller-owned root intact. Clear all framework-local ID and callback maps.

## Required Tests

Add the adapter to `crates/adapter-conformance`. The shared scenario must prove
the same observable tree and protocol behavior for:

1. Initial mount.
2. One event with opaque payload bytes.
3. One query and Result value.
4. One optional capability gap.
5. Explicit destroy and stale-ID cleanup.

Also add one real framework runtime fixture. Directly calling adapter methods is
not sufficient: Yew uses a real `NativeRenderer`; Dioxus uses a real
`VirtualDom` in `examples/dioxus-counter`.

Run `./scripts/verify.sh` before publishing an adapter package.
