# Compatibility and support status

This document records what the experimental preview verifies and, equally
importantly, what it does not provide.

## Compatibility matrix

| Component | Revision or target | Status | Evidence |
| --- | --- | --- | --- |
| Yew | `0e4a05472fac4e5fce1befe60fa4a1e43a36b6a3` | Supported only as the patch base used by this prototype | `scripts/bootstrap-yew.sh` validates the base and patch identities; `scripts/verify.sh` runs narrow renderer and macro checks |
| Yew at any other commit or release | Unpinned | Not evaluated; no compatibility claim | The patch must be rebased and reverified before changing the pin |
| Rust | 1.85.0 | Used for repository verification | `rust-toolchain.toml` and the pinned Yew workspace both require Rust 1.85 |
| Generic `ClayBackend` | Host-independent Rust trait | Experimental; only the focused behaviors below are claimed | Patched Yew renderer tests and `examples/counter` recording backend test |
| OSS Lynx | `0df14207cebb060f1bed8de12b64a1119dee8f06` | Source-audited, but incompatible with the required end-to-end runtime | [`docs/oss-lynx-gap.md`](docs/oss-lynx-gap.md) |
| OSS Lynx at another revision | Unpinned | Not evaluated; no compatibility claim | A new public-source audit is required |
| Android Clay host | Stock OSS Lynx at the audited revision | Unsupported and not included | The audited tree has no required Android Clay renderer, host, JNI integration, or build wiring |
| Runnable Lynx app, APK, or launcher | Any platform | Not included | This repository verifies a fixture, not an application package |

## Verified Yew behavior

| Behavior | Evidence and boundary |
| --- | --- |
| Initial render and one `ontap` state update | Patched-Yew counter test verifies the host tree and flush count; the repository fixture records initial and updated text |
| `rendered` and `destroy` lifecycle | One patched-Yew test verifies synchronous first render, one message-driven rerender, and explicit destroy callbacks |
| Destroy callback panic | One patched-Yew test verifies tree and listener cleanup only when the component's `destroy` callback unwinds |
| `value` and `checked` | One patched-Yew test verifies initial `value` strings, `checked=true`, and omission of the host attribute for initial `checked=false`; updates are not tested |
| Direct-parent teardown | One patched-Yew test verifies recursive removal and destruction of a nested tree |
| Selected invalid output | Tests verify pre-mutation rejection of nested components, an explicit `NodeRef`, and a key introduced by an update |
| Macro behavior | Focused macro tests verify typed `ontap` with `clay` and ordinary `ontap` attributes without `clay` |

The tests do not establish general component lifecycle support, recovery from
create/render/update or backend panics, incremental text or attribute mutation,
all VNode/listener combinations, thread safety, or host integration. Nested
components, browser node references, portals, suspense nodes, raw HTML, keyed
reconciliation, and general event payloads are outside the prototype's supported
surface.

[`patches/yew/README.md`](patches/yew/README.md) describes the patch contract and
known unsupported forms; the evidence table above is the compatibility claim
boundary. The implementation is a generic mutation interface. Its name does not
imply that a Lynx Clay platform backend exists in this repository.

## Public Lynx boundary

The stock OSS Lynx audit found three independent blockers at the pinned
revision:

1. No public native renderer ABI suitable for Rust/Yew node mutation and event
   ownership.
2. No usable OSS native static-page runtime/context factory path.
3. No Android Clay renderer and host integration in the public build.

For that reason this preview deliberately excludes:

- a Lynx source checkout or submodule;
- a downstream Lynx patch series;
- a `NativeContext` ABI or bindings for interfaces absent from the audited
  public source;
- Android Java/JNI glue and a Clay renderer host;
- an APK, launcher shortcut, or end-to-end device test.

The complete findings, public citations, and a staged proposal for closing the
gaps are in [`docs/oss-lynx-gap.md`](docs/oss-lynx-gap.md). Until those public
interfaces exist and pass end-to-end tests, stock OSS Lynx runtime compatibility
must remain unsupported.

## Changing a pin

A Yew revision change requires rebasing `patches/yew/series`, validating each
patch identity from a clean checkout, and rerunning `scripts/verify.sh`. A Lynx
revision change requires a new public-source audit; a newer commit must not be
assumed compatible merely because related source files exist.
