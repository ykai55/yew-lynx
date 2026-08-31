# Element bridge guest protocol

`schema/guest_protocol.fbs` is the language-neutral FlatBuffers v4 wire
contract. `src/guest_protocol_generated.rs` is generated and checked in, so a
normal Cargo build does not require `flatc`.

The schema and Rust runtime are pinned to FlatBuffers 25.2.10. Regenerate with
that exact `flatc` version from this directory:

```sh
./regenerate.sh
```

Review generated and golden-fixture changes before committing them. Any wire
incompatibility requires a new protocol version.
