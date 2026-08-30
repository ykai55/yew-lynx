# FlatBuffers Guest Protocol v3

## Problem Statement

The WASM guest protocol currently uses Postcard and derives its wire layout from
Rust Serde types. That format is convenient for Rust-to-Rust communication but
does not provide a language-neutral schema for guest or host implementations in
other languages.

The protocol needs an explicit, versioned schema that can be consumed by
multiple languages without exposing generated FlatBuffers types through the
existing Rust domain interfaces.

## Solution

Replace the Postcard guest protocol v2 with a FlatBuffers guest protocol v3.
The migration is a hard cut: v3 hosts and guests do not support Postcard v2.

Define one FlatBuffers schema with an `LEB3` file identifier and a single
versioned envelope whose message union contains mount requests, event requests,
and guest responses. Keep generated Rust types private behind owned Rust codec
interfaces. The existing WASM ABI allocation, invocation, output descriptor,
and deallocation contract remains unchanged.

## User Stories

1. As a guest implementer, I want a language-neutral schema so that I can implement a compatible guest outside Rust.
2. As a host implementer, I want explicit enum and union values so that generated bindings agree across languages.
3. As a Rust caller, I want to keep using owned bridge domain types so that FlatBuffers buffer lifetimes do not leak into application code.
4. As a runtime maintainer, I want malformed and semantically invalid messages rejected before application code runs.
5. As a protocol maintainer, I want one identifiable envelope so that message kind and protocol version are validated consistently.
6. As a release maintainer, I want generated Rust sources checked in so that normal builds do not require a system `flatc` installation.
7. As a test author, I want stable encoded fixtures and end-to-end coverage so that schema changes cannot silently change the wire contract.

## Implementation Decisions

- FlatBuffers is the selected serialization format. The initial migration is
  for language interoperability, not an end-to-end zero-copy redesign.
- Protocol v3 replaces protocol v2 directly. Do not retain a Postcard fallback
  or add runtime capability negotiation.
- Introduce a dedicated protocol module shared by the WASM guest and WAMR host.
  It owns the schema, generated bindings, protocol request/response types, and
  codecs. Runtime lifecycle and ABI exports remain in the WASM guest module.
- Use a single root envelope with:
  - file identifier `LEB3`;
  - a required semantic `protocol_version` value of `3`;
  - a message union containing `MountRequest`, `EventRequest`, and
    `GuestResponse`.
- Keep FlatBuffers-generated types private. Public decode operations return
  owned Rust protocol/domain values; public encode operations accept those
  values. Generated accessors and buffer lifetimes are not public contracts.
- Model each `Command` variant as a table selected through a command payload
  union. Store commands as a vector of command wrapper tables.
- Model guest success and failure as separate tables selected through a result
  union.
- Represent IDs as unsigned 32-bit scalars on the wire. Decode through the
  existing domain constructors so node, listener, and callback IDs remain
  nonzero.
- Assign explicit stable numeric values to every status and union member.
  Existing values must never be reordered or reused. Schema evolution may only
  append compatible fields and variants; removed fields retain their slots.
- Preserve `SetAttribute.value` semantics: an absent string is `None`, while a
  present empty string is `Some("")`.
- Preserve event payload as opaque bytes and content type as a UTF-8 string.
- Reject buffers with an invalid identifier, invalid FlatBuffers structure,
  wrong protocol version, wrong message kind for the called codec, missing
  union payload, unknown enum/union values, zero IDs, or an error result whose
  status is `Ok`.
- Keep the current WASM ABI exports, synchronous call model, buffer descriptor,
  and input/output ownership rules unchanged. The exported `version()` value
  becomes `3`.
- Check the schema and generated Rust source into the repository. Normal Cargo
  builds must not invoke `flatc`. Document the exact generator/runtime version
  and provide a reproducible regeneration command or script.
- Remove Postcard and protocol-only Serde dependencies and derives after all v3
  call sites have migrated. Do not leave dormant v2 codecs.
- Update compatibility documentation to identify the WASM guest protocol as
  FlatBuffers version 3.
- Keep the encoder implementation infallible where FlatBuffers construction has
  no recoverable error. Do not introduce an artificial public error abstraction
  merely to preserve Postcard's old return type; update callers deliberately.

## Testing Decisions

- Treat the owned codec interface as the primary protocol test seam. Tests
  assert externally visible round trips and validation behavior, not generated
  accessor details.
- Round-trip mount, event, successful response, error response, and every
  command variant. Include `None`, `Some("")`, arbitrary payload bytes, empty
  command vectors, and all status values where valid.
- Add negative tests for corruption, wrong identifier, wrong version, wrong
  envelope message kind, missing union payloads, zero IDs, unknown enum/union
  values, truncated buffers, and `Ok` used as an error status.
- Add deterministic golden byte fixtures for representative mount, event,
  success, and error messages. Golden fixtures are protocol compatibility
  evidence and should only change with an intentional protocol version change.
- Retain and adapt the existing Guest ABI lifecycle tests to verify allocation,
  deallocation, mount, event, destroy, and error responses through v3 codecs.
- Retain and adapt the real WAMR lifecycle tests, including malformed guest
  output and unsupported-version behavior.
- Run workspace formatting, workspace tests, Clippy under the pinned Rust
  toolchain, and the existing real WAMR tests supported by the repository
  environment.
- The first release does not require a non-Rust generated-binding smoke test.
  Cross-language execution coverage can be added separately without changing
  the v3 schema.

## Out of Scope

- Supporting Postcard protocol v2 after migration.
- Runtime protocol negotiation or multiple simultaneous codecs.
- Changing the native renderer FFI protocol.
- Changing the development server JSON reload protocol.
- Serializing `TreeSnapshot` or other core types not currently crossing the
  WASM guest protocol.
- Redesigning WAMR memory ownership for zero-copy access.
- Publishing or testing Kotlin, C++, TypeScript, or other non-Rust generated
  bindings in the initial migration.
- Adding new command semantics, asynchronous calls, or session transport.

## Further Notes

- FlatBuffers structural verification does not enforce bridge domain
  invariants. The wire-to-domain conversion is mandatory even when structural
  verification succeeds.
- FlatBuffers may produce larger messages than Postcard. This is accepted for
  v3 because interoperability is the primary goal. Performance tuning requires
  measurements and is not part of this migration.
- The protocol module should remain a deep module: callers learn the owned
  encode/decode interface, while schema construction, verification, generated
  types, and conversion logic remain local to its implementation.
