# MTS broker protocol v2

The broker is the only owner of Lynx `ElementRef` values and event callbacks.
Rust and MTS exchange verified FlatBuffers envelopes with file identifier
`LEB2`; JSON and decimal-string IDs are not part of the wire protocol.

## Native Methods

All native module methods are synchronous:

- `invoke("mount", rootId)` returns a Command envelope and establishes the
  nonzero 32-bit session ID carried by that batch.
- `invoke("dispatchEvent", eventBytes)` accepts an Event envelope and returns a
  Command or failure Result envelope.
- `invoke("completeBatch", resultBytes)` accepts a Result envelope containing
  host query results and returns the same Result as an acknowledgement, or a
  failure Result envelope.
- `invoke("destroySession")` returns teardown commands or a failure Result
  envelope. The distinct name avoids Java's inherited `void destroy()` hook.

Sessions, nodes, listeners, callbacks, result slots, and sequences are unsigned
32-bit values. Protocol IDs are nonzero and opaque; only result slot zero is
valid where the schema permits it.

## Channels

- `COMMAND` carries an ordered `CommandBatch`. The broker requires
  `final_commit=true`, validates the complete batch before mutation, then adds
  one host flush at that commit boundary.
- `RESULT` carries a `ResponseBatch`. Batch-level failures preserve their
  `Status`; item-level results return typed values or `UNSUPPORTED` for optional
  capability gaps.
- `EVENT` carries an `EventMessage` with session, listener, callback, content
  type, and opaque payload bytes. The current Lynx host serializes its event
  object as `application/json`; native code receives those bytes unchanged.

The broker validates a successful completion acknowledgement against the sent
session and sequence. A rejected or mismatched acknowledgement poisons the
broker because native and host state can no longer be assumed to agree.

The schema and generated bindings live under `protocol/`. The MTS reader rejects
buffers without the `LEB2` identifier, mismatched channel/message pairs,
unsupported versions, and malformed command ownership.

## Host Execution

Core mutations create, insert, remove, release, update attributes, and manage
listeners through the public Fiber API. Other declarations are decoded by the
generated Element API dispatcher. The revision capability manifest determines
whether each typed operation is available; unavailable optional operations
produce an item-level `UNSUPPORTED` result instead of invoking a missing host
global.

Raw text may only be attached directly beneath a `text` element. Node ownership,
listener identity, callback identity, references, result IDs, and the final
flush boundary are checked before any host call. Invalid batches therefore make
no Fiber mutation. A host exception after validation poisons the broker because
Fiber calls are not transactional; destroy remains available for best-effort
cleanup.

An event callback synchronously encodes one Event envelope, invokes
`dispatchEvent`, and applies the returned command batch before returning. Nested
batch application, event dispatch, and destroy are rejected while another
broker operation is active.

## Lifecycle

Initial render validates the final commit but suppresses its explicit flush
because Lynx's enclosing Fiber pipeline flushes after `__RenderPage` returns.
Update/reload remount, event, explicit batch, and destroy paths flush normally.

Destroy validates native cleanup, applies valid teardown, removes listeners and
attachments left by incomplete cleanup, flushes once, and clears all registries.
Native failures retain their status after host cleanup. Cached roots, nonempty
cache data, and SSR hydration are rejected because the broker cannot adopt
`ElementRef` values it did not create.
