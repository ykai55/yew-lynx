# MTS broker protocol v1

The broker is the only owner of Lynx `ElementRef` values and event callbacks.
The native module and its Rust consumer use positive JavaScript safe-integer IDs
only. The page root is ID `1` in the supplied shell.

All native module methods are synchronous and match the stock Android adapter.
IDs cross MTS as decimal strings so the Java bridge never receives a lossy
floating-point value:

- `invoke("mount", String(rootId))` returns the initial response JSON string.
- `invoke("dispatch", String(listenerId), "tap")` returns a response JSON string.
- `invoke("destroySession")` returns a cleanup response JSON string. The distinct
  name avoids the stock Java module lifecycle method's `void destroy()` return.

Every response has exactly one of these outer shapes:

```json
{"version":1,"ok":true,"operations":[{"op":"flush","root":1}]}
{"version":1,"ok":false,"status":6,"error":"invalid listener","operations":[]}
```

Success responses must contain a valid operation sequence ending in exactly one
`flush`. Failure responses normally have an empty operation array. Only a
destroy failure may carry a nonempty, valid cleanup sequence, and that sequence
must also end in exactly one `flush`. No other fields are accepted. The broker
throws native failures as `E_NATIVE` while preserving the numeric `status`.

The broker validates every mutation against a cloned ownership model before it
calls any host API. Fields not listed below are rejected.

| Operation | Fields | Meaning |
| --- | --- | --- |
| `create_element` | `node`, `tag` | Create `view`, `text`, `image`, and `scroll-view` through their specialized Fiber APIs, or another authored tag through the generic Fiber API. Structural and list-like tags are rejected. |
| `create_text` | `node`, `text` | Create a raw-text ElementRef. Raw text may only be inserted under a `text` element. |
| `insert_before` | `parent`, `child`, `reference` | Attach a detached child. A null `reference` appends; otherwise the reference must be a direct child of `parent`. |
| `remove` | `parent`, `child` | Detach a direct child without releasing its ID. |
| `destroy_node` | `node` | Release a detached, childless node ID after all of its listeners have been removed. The Fiber tree owns actual node lifetime. |
| `set_attribute` | `node`, `name`, `value` | Set or clear an attribute. `id`, `class`, and `style` route to their dedicated Fiber functions. `value` is a string or null. |
| `add_event_listener` | `node`, `listener`, `name` | Register one callback. Protocol v1 accepts only `tap`. |
| `remove_event_listener` | `node`, `listener` | Remove the exact registered callback synchronously. |
| `flush` | `root` | Flush the registered root. It must be the only flush and the final operation. |

For attribute removal, `null` maps to the sentinel required by each public
Fiber API: `""` for ID, class, and inline style, and `undefined` for a
general attribute.

Example:

```json
{
  "version": 1,
  "ok": true,
  "operations": [
    {"op":"create_element","node":2,"tag":"text"},
    {"op":"create_text","node":3,"text":"Count: 0"},
    {"op":"insert_before","parent":2,"child":3,"reference":null},
    {"op":"insert_before","parent":1,"child":2,"reference":null},
    {"op":"flush","root":1}
  ]
}
```

Every accepted success response ends with exactly one explicit tree flush. A no-op response
therefore contains only its final `flush` operation.
An invalid response performs no Fiber mutation and no flush. A host failure after
validation poisons the broker because Fiber operations are not transactional;
only `destroy` remains available for best-effort cleanup.

An event callback synchronously invokes `dispatch(listenerId, "tap")` and applies
the returned batch before returning. Nested batch application, nested event
dispatch, and destroy during another broker operation are rejected.

Destroy first validates the complete response. It applies valid cleanup from
either a success or failure response, removes listeners or attachments left by
incomplete cleanup, flushes once, and then throws any native failure with its
status. Registries are cleared even when the native cleanup response is invalid
or a host cleanup operation fails.

The initial render mount validates the protocol's final `flush` but does not
execute it because Lynx's enclosing Fiber pipeline flushes after `__RenderPage`
returns. Update/reload remount, event, explicit batch, and destroy paths retain
their normal flush boundary.
