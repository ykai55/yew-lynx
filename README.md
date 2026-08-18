# yew-lynx

> [!WARNING]
> **Experimental public preview.** This repository is an independent research
> prototype, not an officially supported Yew or Lynx integration. It currently
> provides a generic Yew `ClayBackend` patch and a host-independent recording
> test fixture. It does **not** provide a runnable Lynx application.

Stock OSS Lynx at the audited revision
[`0df14207cebb060f1bed8de12b64a1119dee8f06`](https://github.com/lynx-family/lynx/tree/0df14207cebb060f1bed8de12b64a1119dee8f06)
lacks the public native runtime, renderer ABI, and Android Clay host needed to
run this prototype. Consequently, this repository includes no APK, launcher,
`NativeContext` ABI, Java/JNI glue, Android host, Lynx patch, or Lynx submodule.
See the [full OSS Lynx gap audit](docs/oss-lynx-gap.md).

## Status

| Area | Status |
| --- | --- |
| Yew base | Pinned to exactly [`0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3`](https://github.com/yewstack/yew/tree/0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3) |
| Yew patch | Experimental generic `ClayBackend` renderer with the narrow tests listed below |
| Rust fixture | Host-independent counter recording initial text, one tap update, explicit cleanup, and flushes |
| Stock OSS Lynx runtime | Unsupported at the audited revision |
| Android application | Not included and not currently buildable from this repository |
| Support level | Experimental preview; no official support or stability commitment |

The Yew patch is tested only against the exact commit above. Compatibility with
other Yew commits or releases has not been established. See
[COMPATIBILITY.md](COMPATIBILITY.md) for the complete matrix and scope boundary.

## Verified scope and limitations

The automated evidence is deliberately narrower than the backend API:

- Patched-Yew tests cover rejection before host mutation for selected unsupported
  trees, initial `value` and `checked` attribute forwarding, synchronous
  `rendered` and `destroy` callbacks, direct-parent teardown, unwind cleanup when
  the component's `destroy` callback panics, and one `ontap` state update.
- The counter fixture separately records initial and updated text, listener
  removal during explicit destroy, and flush boundaries. It does not model a
  host node tree, attributes, layout, rendering, threads, or host failures.
- No test establishes general lifecycle panic recovery, incremental text or
  attribute updates, keyed reconciliation, nested components, event payloads,
  a native host, or device behavior.

Only this tested behavior is claimed. The exposed backend trait is experimental,
and untested operations or combinations must not be treated as supported.

## Included

- `patches/yew/`: a format-patch series adding an opt-in, backend-driven Yew
  renderer for a deliberately narrow VNode and event surface.
- `examples/counter/`: a Rust fixture that implements only `ClayBackend`. Its
  recording test checks initial text, one counter update, listener cleanup on
  explicit destroy, and flushes without depending on Lynx platform code.
- `scripts/bootstrap-yew.sh`: a reproducible bootstrap for the exact Yew base
  and patch series.
- `scripts/verify.sh`: the local and CI verification entry point.
- `docs/oss-lynx-gap.md`: the public-source audit explaining why no runnable
  Lynx integration is present.

The `ClayBackend` API is a host contract, not a Lynx implementation. A consumer
would still need a supported public native runtime and renderer binding before
it could render through Lynx.

## Prerequisites

- Git with network access to <https://github.com/yewstack/yew>
- Rust 1.85.0 and the `rustfmt` component (also declared in
  `rust-toolchain.toml`)
- Bash
- Optional: ShellCheck, which `scripts/verify.sh` runs when available

## Bootstrap

From the repository root:

```bash
./scripts/bootstrap-yew.sh
```

The script clones the pinned Yew revision into the ignored `.deps/yew`
directory and applies every patch listed in `patches/yew/series`. Re-running it
is safe when the checkout already contains exactly that series. If the checkout
has uncommitted work, a branch checkout, an unexpected commit, a different
history, or an interrupted `git am`, the script exits without resetting or
overwriting the checkout.

## Verify

```bash
./scripts/verify.sh
```

Verification performs:

1. Bash syntax checks and ShellCheck when installed.
2. Idempotent Yew bootstrap and patch identity validation.
3. Project formatting, workspace checks, and workspace tests.
4. A patched-Yew `clay` feature check.
5. Narrow Yew Clay renderer tests.
6. Narrow Yew macro tests both with and without the `clay` feature.

These checks validate the generic renderer prototype and recording fixture
only. They do not validate a Lynx runtime, Android renderer, application
package, or device behavior.

## Panic and cleanup limits

This workspace intentionally does not override Cargo's release panic strategy,
so unwind cleanup can run in repository builds. The focused panic test verifies
only that a panicking component `destroy` callback still removes the tested tree
and listener state when unwinding is enabled.

Cleanup is not guaranteed if a downstream build selects `panic = "abort"`;
aborting terminates the process without running unwind guards. Cleanup is also
not guaranteed if a host callback or `ClayBackend` method panics, because a
second panic during unwind can abort the process. Backends must therefore keep
all methods and callbacks non-panicking, and consumers must call
`ClayAppHandle::destroy` explicitly rather than relying on handle drop.

## Licensing

This repository is licensed under the [Apache License 2.0](LICENSE). Yew is
available under MIT or Apache-2.0, and the cited Lynx source is Apache-2.0. See
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for exact revisions and
attributions.
