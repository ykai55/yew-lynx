# Stock OSS Lynx gaps for a proposed native Yew static page

## Executive summary

This investigation audited the official public
[`lynx-family/lynx`](https://github.com/lynx-family/lynx) repository at commit
[`0df14207cebb060f1bed8de12b64a1119dee8f06`](https://github.com/lynx-family/lynx/tree/0df14207cebb060f1bed8de12b64a1119dee8f06),
which was the `develop` branch HEAD on 2026-08-18.

**Stock OSS Lynx at this revision cannot provide the proposed Android Yew
target.**

The public tree contains useful pieces, but not an end-to-end native static-page
runtime:

| Requirement | Stock OSS status | Result |
| --- | --- | --- |
| Stable native renderer ABI for a Rust/Yew VDOM backend | Missing | No public C ABI can create and reconcile Lynx nodes, bind events, and flush the render pipeline. |
| Android static-page lifecycle host | Present but partial | Public Java APIs can attach an already-created static page instance and route data/lifecycle calls. |
| Native static-page runtime registration/factory | Missing | The OSS factories explicitly return no RTS or RTS Native context/bundle, so an RTS Native template cannot be decoded and executed. |
| Android Clay renderer | Missing | Public Clay core and common Lynx adaptor code exist, but the Android Clay renderer, engine bridge, and Android build wiring do not. |
| Proposed Android target for the counter fixture | Unsupported | All three layers are required before the proposed target can run. |

No Lynx patch is included in this repository. A small factory-hook-only patch
would not make the proposed target usable, while implementing the renderer ABI or Android
Clay integration requires new public API design and substantial platform code
that cannot be reconstructed confidently from the stock public tree alone.

## Scope and public-source policy

The compatibility conclusions and source citations in this document come only
from the official public Lynx repository at the pinned commit above.

The counter fixture was used only to derive the proposed capability contract.
All Lynx-specific findings come from the cited upstream implementation; no Lynx
source, build artifacts, or patches were copied into this repository.

The Yew-side patch is a separate publication and licensing concern. This audit
does not establish that the patched Yew dependency is upstream-compatible or
ready for publication; it only evaluates whether stock OSS Lynx can host the
resulting native application.

## Required capability contract

The proposed target based on the counter fixture is intentionally small, but it
still requires all of the following host capabilities.

### Native runtime lifecycle

1. Load a template that selects a native/static runtime rather than a JavaScript
   runtime.
2. Resolve a statically linked or registered application factory.
3. Invoke that factory on the Lynx runtime owner thread with an opaque, valid
   context handle.
4. Dispatch initial render, metadata update, and destruction in a defined order.
5. Destroy the application synchronously before invalidating the context.
6. Reject duplicate mounting and calls made against stale handles.

### Renderer operations

The proposed target needs these backend operations:

1. Obtain the page root.
2. Create authored elements and raw text nodes.
3. Insert, move, remove, and destroy nodes.
4. Set, update, and clear attributes, including inline style, identifiers, and
   accessibility properties.
5. Update raw text after component state changes.
6. Register and remove a tap listener with deterministic callback lifetime.
7. Flush a reconciliation batch through the Lynx render pipeline.
8. Keep all node, listener, and context handles valid only on the documented
   owner thread.

### Android renderer

The proposed target requests the Clay renderer on Android. It therefore needs
an Android rendering shell that owns a Clay `ViewContext`,
constructs Lynx's `UIDelegateClay`, connects Android input and lifecycle, and
presents frames through an Android surface or texture.

These are independent layers. The Java static-page host cannot replace the
native renderer ABI, and the presence of public Clay core code cannot replace an
Android Clay shell.

## Finding 1: no public native VDOM renderer ABI

### What is public

Stock Lynx has several APIs with "renderer" in their names, but they solve
different problems.

The Android SDK exposes a whole-renderer creator interface,
[`IUIRendererCreator`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/tasm/IUIRendererCreator.java#L9-L31),
and its API baseline lists that interface as public
([API baseline](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/api/lynx_android.api#L3259-L3269)).
Its product is the large Java
[`ILynxUIRenderer`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/tasm/behavior/ILynxUIRenderer.java#L29-L138)
interface. The stock creator returns the ordinary Java renderer
([`LynxUIRendererCreator`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/tasm/behavior/LynxUIRendererCreator.java#L8-L12)).
This is a Java platform-renderer replacement seam, not an ABI through which a
Rust VDOM can mutate the Lynx element tree.

The desktop embedder also exposes a genuine C API for a
[`lynx_windowless_renderer_t`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/embedder/public/capi/lynx_windowless_renderer_capi.h#L66-L98).
Its callbacks provide frame presentation, graphics context, host task runner,
input, and clipboard integration
([presentation callbacks](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/embedder/public/capi/lynx_windowless_renderer_capi.h#L103-L173)).
It is an output-surface/embedder API. It does not expose node creation,
attributes, text reconciliation, or Lynx event-listener registration.

The upstream implementation has element mutation functions for its script
runtimes, for example in
[`core/runtime/lepus/bindings/renderer.h`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/runtime/lepus/bindings/renderer.h#L24-L61).
Those are C++ bindings using upstream implementation value and runtime types,
not a versioned public C ABI suitable for direct Rust bindings.

### What is missing

The public tree has no `core/runtime/rts` implementation and no exported
`lynx_rts_renderer_*` API. It also has no public opaque node/listener handles,
ABI version negotiation, ownership rules, or callback teardown contract for a
native static page.

The Android fragment/platform-renderer classes are likewise not a replacement.
They implement display-list/platform-view rendering for Lynx-managed elements;
they do not provide a page-level VDOM mutation ABI to an external runtime.

### Verdict

Stock OSS Lynx has public renderer extension points, but **does not have the
kind of public native renderer ABI required by the proposed target**.

## Finding 2: static-page host APIs exist, but the runtime factory does not

### Public Android host API

Android has a meaningful public static-page lifecycle layer.

[`StaticPageHost.attach`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/tasm/StaticPageHost.java#L19-L46)
binds an already-created `StaticPageInstance` to an instance ID.
[`StaticPageInstance`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/tasm/StaticPageInstance.java#L11-L18)
defines render, metadata update, and destroy callbacks. Both are present in the
published Android API baseline
([baseline entries](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/api/lynx_android.api#L10116-L10123)).

The host stores data, routes metadata updates, and runs destruction through its
owner executor. Its native-facing render entry calls the attached page instance
([native render entry](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/tasm/StaticPageHost.java#L159-L163),
[`renderPage`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/tasm/StaticPageHost.java#L194-L212)).
The Android data API can retain a platform map for static-page loading
([`TemplateData.createForStaticPage`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/tasm/TemplateData.java#L144-L159)).

This layer is useful and should be preserved in a proposed upstream
implementation.

### Native runtime path is absent from the upstream implementation

The public `ContextType` enum includes RTS and RTS Native values
([`mts_context.h`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/runtime/mts_context.h#L36-L41)),
but the OSS factory groups both values into an unsupported branch and returns
`nullptr`
([`MTSContextFactory::Create`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/runtime/mts_context_factory.cc#L16-L37)).
The corresponding context-bundle factory also returns `nullptr`
([`ContextBundleFactory::Create`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/runtime/mts_context_factory.cc#L45-L61)).

The template codec boundary is incomplete as well. The public header declares
RTS and RTS Native magic constants
([`magic_number.h`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/template_bundle/template_codec/magic_number.h#L13-L19)),
but the OSS definition file defines only the ordinary QuickJS and Lepus magic
values
([`magic_number.cc`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/template_bundle/template_codec/magic_number.cc#L10-L15)).
The stock decoder correspondingly accepts only those two values
([`DecodeMagicWord`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/template_bundle/template_codec/binary_decoder/lynx_binary_base_template_reader.cc#L22-L41)).
Adding a context factory alone would therefore still not complete the native
template path.

This fails before application factory lookup. Template decoding creates a
context bundle and requires it to be non-null
([`LynxBinaryReader::DecodeContext`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/template_bundle/template_codec/binary_decoder/lynx_binary_reader.cc#L253-L259)).
The public factory header exposes only static `Create` functions and no provider
registration API
([`mts_context_factory.h`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/runtime/mts_context_factory.h#L16-L29)).

`ContextBundle::OnCustomSectionDecoded` is a useful virtual hook
([`mts_context.h`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/runtime/mts_context.h#L50-L61)),
but it is not a runtime provider seam: the OSS bundle factory returns null before
there is an object on which to invoke the hook.

Harmony contains a static-task N-API loader, but it does not change the Android
result. More importantly, its initialization is compiled only when several
headers absent from the OSS tree are available
([compile guard](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/harmony/lynx_harmony/src/main/cpp/lynx_napi_export.h#L29-L41),
[conditional initialization](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/harmony/lynx_harmony/src/main/cpp/lynx_napi_export.h#L64-L69)).

### What `StaticPageHost.attach` does not do

`attach` accepts an application instance that somebody else has already
created. It does not:

1. Decode a native application descriptor from a template.
2. Resolve a static-library symbol or registered factory.
3. Create an RTS Native context.
4. Expose renderer operations to that context.
5. Invoke a Rust application factory.

### Verdict

The Android static-page host contract is public and reusable, but **stock OSS
Lynx has no native static-page runtime registration/factory seam**. An RTS Native
template cannot reach the host lifecycle with the OSS factories as shipped.

## Finding 3: Android Clay is not in the public build

### Reusable public Clay code

The repository contains substantial public Clay engine code and a common Lynx
adaptor. In particular,
[`UIDelegateClay`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/clay/lynx_adaptor/ui_delegate_clay.h#L28-L74)
adapts a Clay `ViewContext` into Lynx painting, layout, property, and event
interfaces.

That common adaptor is used by public desktop/windowless integrations. The
windowless renderer creates a `ClayHeadlessEngine`, obtains its `ViewContext`,
and constructs `UIDelegateClay`
([windowless integration](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/embedder/windowless/lynx_ui_renderer_windowless.cc#L82-L98)).
This demonstrates that the common Clay layer is real, but it does not supply an
Android window, surface, or Java renderer.

### Missing Android layer

Clay is disabled by default
([`enable_clay = false`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/clay/common/config.gni#L10-L18)).
The root `enable_clay` branch adds only `clay:standalone_lib`
([root `BUILD.gn`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/BUILD.gn#L35-L41)),
and that target is marked `testonly` and depends on the GLFW example
([`clay/BUILD.gn`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/clay/BUILD.gn#L114-L137)).
It is not the Android SDK renderer.

The stock Android aggregate build depends on Lynx Android, DevTool, base, trace,
and graphics modules, with no Clay renderer target
([`platform/android/BUILD.gn`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/BUILD.gn#L23-L30)).
The Android library's Gradle dependency block likewise has no Clay module or
prefab dependency
([`lynx_android/build.gradle`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/build.gradle#L312-L348)).

There is no `platform/android/lynx_clay` directory and no Android
`LynxUIRendererClay` implementation in the audited tree. A public Clay design
document itself identifies the concrete Android bridge as living in that absent
directory
([native-view specification](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/clay/ui/component/spec/native_view_spec.md#L123-L134)).
The API types `ClayRenderMode`, `ILynxClayService`, and Clay-related default
methods are compatibility surfaces for an optional implementation; they do not
instantiate one.

### Verdict

**Stock OSS Lynx does not support the Clay renderer on Android at this
revision.** Public Clay core code is insufficient without the Android engine,
Java renderer, JNI bridge, native-view bridge, packaging, and lifecycle code.

## Could the stock Android Java renderer be used instead?

The counter's visible UI uses basic `view`, `text`, inline styles,
accessibility attributes, and tap handling. Those features are available in the
ordinary Android Lynx renderer, so a separate non-Clay experiment might be
possible after a public native runtime and renderer ABI exist.

That would not be compatibility with the proposed Clay target. It also does not
remove the two primary blockers: stock OSS still cannot create an RTS Native
context, and Rust still has no public node/event ABI.

Using the ordinary renderer is therefore a possible scope reduction for an
independent proof of concept, not a workaround that makes the proposed target
run.

## Why no Lynx patch is provided

No file was created under `patches/lynx/` for this audit.

A clean patch must have a reviewable contract, tests, and a useful behavior
change. The following tempting partial changes do not meet that bar:

| Partial change | Why it is not sufficient |
| --- | --- |
| Add one callback to `MTSContextFactory` | Decoding still needs a context bundle, there is no task descriptor contract, and there is no renderer ABI. |
| Export existing C++ renderer functions directly | Their signatures and ownership use upstream implementation C++/Lepus objects and would create an unstable, unsafe ABI. |
| Call `StaticPageHost.attach` from application Java | There is still no native context or factory invocation from the template. |
| Set `enable_clay=true` on Android | The Android Clay renderer and shell targets do not exist in the public tree. |
| Reuse the windowless Clay C API | It hosts frame presentation for the desktop embedder and does not provide Android view integration or a VDOM node API. |

A format patch containing only one of these changes would imply progress toward
a runnable configuration while leaving the same hard failure in place. The
correct next artifact is an upstream API proposal followed by independently
reviewable implementation patches.

## Exact public implementation plan

The plan below is intentionally staged so every patch has a testable behavior
and can be proposed upstream using only the cited upstream source.

### Phase 0: freeze the public contracts

Define two public, versioned contracts before implementation:

1. A native static-page application contract that maps a template descriptor to
   a registered factory and lifecycle callbacks.
2. A renderer C ABI that lets the application mutate a Lynx element tree
   without exposing C++ objects or Lepus values.

The ABI should use opaque handles, fixed-width integer types, `struct_size` and
`abi_version` fields, explicit status codes, and host-owned UTF-8 byte spans.
Every function must state owner-thread requirements and whether handles are
borrowed, retained, consumed, or invalidated.

The API must define these failure cases up front: unsupported ABI version,
unknown application ID, duplicate registration, duplicate mount, wrong thread,
stale node/listener/context handle, callback panic/exception, and host teardown
during a callback.

Suggested public headers:

```text
core/public/native_page_runtime_capi.h
core/public/native_page_renderer_capi.h
```

The exact names require upstream agreement. They are proposals, not existing
APIs.

### Phase 1: add a provider-backed native context and bundle

Implement the missing stock runtime path using only public repository code.

1. Add a process-level provider registration API with register-once semantics.
2. Teach `ContextBundleFactory` to create an OSS native-page bundle when the
   provider is installed.
3. Store a versioned application descriptor in that bundle through the existing
   custom-section decode hook.
4. Teach `MTSContextFactory` to ask the provider for a native-page context.
5. Preserve the existing explicit unsupported error when no provider is
   installed.
6. Ensure decoding malformed descriptors fails before any application code is
   invoked.
7. Keep registration independent of Android so host applications and other
   platforms can provide factories consistently.

Primary existing files affected:

```text
core/runtime/mts_context.h
core/runtime/mts_context_factory.h
core/runtime/mts_context_factory.cc
core/template_bundle/template_codec/binary_decoder/lynx_binary_reader.cc
core/template_bundle/template_codec/magic_number.h
core/template_bundle/template_codec/magic_number.cc
```

New runtime implementation should live in a public OSS-owned directory such as
`core/runtime/native_page/`, with no conditional dependency on absent source.

Phase 1 tests must cover context and bundle creation with and without a provider,
descriptor decoding, duplicate registration, concurrent reads after
registration, unsupported ABI versions, and deterministic error reporting.

### Phase 2: implement the renderer C ABI

Implement a C adapter over Lynx's existing element and pipeline behavior rather
than exposing upstream implementation binding objects.

The minimum operation set for the counter fixture is:

```text
get_root
create_element
create_text
insert_before
remove_child
destroy_node
set_text
set_attribute
remove_attribute
add_event_listener
remove_event_listener
flush
```

The event callback must receive an opaque event handle plus a user-data pointer.
The first version may expose only the event name, but the ABI must allow later
event fields without changing existing struct layout. Listener removal and
context destruction must prevent callbacks into freed Rust state.

Node operations should enqueue normal Lynx mutations and use the normal pipeline
flush path. They must not call platform UI objects directly. This keeps the ABI
renderer-neutral and allows both the standard Android renderer and Clay to use
the same native page.

Phase 2 tests must include a pure C consumer, a Rust FFI smoke consumer, node
ownership misuse, insertion/move ordering, attribute clearing, text update,
listener removal, reentrant state update from a tap callback, and destruction
with queued work.

### Phase 3: connect runtime lifecycle to the existing Android host

Reuse `StaticPageHost` and `StaticPageInstance` rather than replacing them.

1. Add an SDK-owned Android `StaticPageInstance` adapter backed by the native
   context.
2. Register that adapter with `StaticPageHost.attach` after context creation and
   before initial render can be requested.
3. Route `renderPage` to the registered native application render callback on
   the context owner thread.
4. Route metadata snapshots through an explicit value ABI; do not pass JNI or
   Lepus implementation pointers into Rust.
5. Route `destroy` synchronously, remove all event listeners, invalidate every
   borrowed handle, then release the native context.
6. Make failed factory resolution fail page load with a public diagnostic rather
   than falling back to a partially initialized standard runtime.

Primary Android areas:

```text
platform/android/lynx_android/src/main/java/com/lynx/tasm/StaticPageHost.java
platform/android/lynx_android/src/main/java/com/lynx/tasm/StaticPageInstance.java
platform/android/lynx_android/src/main/jni/
platform/android/api/lynx_android.api
```

Phase 3 tests must extend the existing `StaticPageHostTest` with native adapter
coverage and add integration cases for initial data, global props, update before
load, update after load, repeated load rejection, destroy-before-queued-update,
and factory failure.

### Phase 4: choose and implement the Android rendering target

There are two valid routes. They should not be mixed in one patch series.

#### Route A: standard Android renderer first

Use the ordinary Java renderer to validate Phases 1 through 3 with the counter.
This is the smallest public proof of the native runtime and renderer ABI. It
does not claim Clay compatibility.

Exit criterion: the counter renders, increments on tap, and is destroyed and
remounted without stale state using a stock Android Lynx UI renderer.

#### Route B: public Android Clay support

For the proposed Android Clay target, publish or independently implement the
missing layer. The implementation needs all of the following:

1. An Android Clay engine/shell that owns task runners, a `ViewContext`, and
   surface or texture presentation.
2. A Java `ILynxUIRenderer` implementation and matching
   `IUIRendererCreator`.
3. JNI ownership connecting the Java renderer to the Clay engine and
   `UIDelegateClay`.
4. Android touch, accessibility, lifecycle, resize, foreground/background, and
   screenshot integration.
5. The native-view bridge described by the public Clay specification.
6. Resource loading, image decoding, font setup, graphics backend, and ICU setup.
7. GN targets, Gradle/CMake integration, prefab or source linkage, packaging,
   proguard/keep rules, and ABI filters.
8. Android CI that builds and runs Clay rather than only compiling public API
   stubs.

The preferred route is for the Lynx project to publish its Android Clay
integration under the repository's Apache-2.0 license. An implementation based
on the available common Clay interfaces is possible in principle, but it is a
large platform project and should not be represented as a compatibility patch
until it has rendering, lifecycle, and device test coverage.

### Phase 5: migrate the Yew backend to the public ABI

After the Lynx ABI exists:

1. Generate or hand-maintain Rust declarations from the public C headers.
2. Replace any direct C++ context assumptions with the opaque public context
   handle.
3. Map Yew node/listener handles to the documented Lynx ownership model.
4. Catch Rust panics at every exported callback boundary.
5. Register the counter factory through the public application registry.
6. Keep the Yew patch independently reproducible from its public upstream
   revision.

Do not make Yew depend on headers outside the public upstream API, C++ mangled
symbols, raw Lepus values, or an unversioned function table.

### Phase 6: end-to-end acceptance

Android support should remain marked unsupported until all of these checks pass:

| Check | Required result |
| --- | --- |
| OSS source provenance | Every added line is authored from public interfaces or newly designed in public review. |
| C ABI compile | C11 and C++ consumers compile; Rust declarations match size/alignment and symbol names. |
| Stock renderer E2E | Counter shows `0`, tap shows `1`, close/reopen resets to `0`. |
| Clay renderer E2E | Same behavior on a real Android arm64 device using the public Clay implementation. |
| Lifecycle stress | Repeated mount/destroy, background/foreground, and activity recreation have no stale callbacks. |
| Sanitizers | ASan/UBSan report no use-after-free or invalid callback access. |
| Thread checks | Every ABI call fails predictably off the owner thread. |
| API baseline | Android and native public API metadata are updated and reviewed. |
| Patch reproducibility | Every format patch applies to its declared base SHA in a clean clone. |

## Suggested upstream patch sequence

Keep the work as independent upstream-reviewable changes:

1. Specify the native-page descriptor, provider, threading, and ownership model.
2. Add provider-backed OSS context-bundle creation and failure tests.
3. Add provider-backed native context creation and lifecycle tests.
4. Add the versioned renderer C ABI and C/Rust tests.
5. Connect the Android `StaticPageHost` adapter and Android integration tests.
6. Prove the counter with the standard Android renderer.
7. Add the Android Clay shell and Java/JNI renderer in separately reviewable
   platform patches.
8. Enable Android Clay counter E2E and sanitizer coverage.

Avoid a single downstream mega-patch. It would combine ABI design, runtime
ownership, Android lifecycle, and graphics integration, making both public
review and future rebases unsafe.

## Lynx submodule recommendation

**Do not add a Lynx submodule to this repository yet.** Stock Lynx at the
audited revision cannot run the proposed target, and a submodule would add a large build
dependency while suggesting a working integration that does not exist.

When the first public Lynx implementation patch is ready, add the official
upstream repository as a development/build submodule rather than another fork:

```text
URL:      https://github.com/lynx-family/lynx.git
Path:     third_party/lynx
Revision: 0df14207cebb060f1bed8de12b64a1119dee8f06
```

Use the full commit SHA, not the moving `develop` branch. Apply a public
`patches/lynx/` format-patch series in CI and verify its declared base before
building. If upstream accepts the required implementation, move the submodule
to the upstream merge commit or a release containing it and remove the
downstream patches.

The audited SHA is a reproducibility baseline, not a claim that it is a stable
release. Re-audit any newer revision before changing the pin because this area
is actively evolving.

## Reproduction commands

The following commands reproduce the source audit using only the official
upstream repository:

```bash
git ls-remote https://github.com/lynx-family/lynx.git HEAD refs/heads/develop
git clone --depth 1 --filter=blob:none \
  https://github.com/lynx-family/lynx.git lynx-oss
git -C lynx-oss fetch --depth 1 origin \
  0df14207cebb060f1bed8de12b64a1119dee8f06
git -C lynx-oss checkout --detach \
  0df14207cebb060f1bed8de12b64a1119dee8f06
git -C lynx-oss rev-parse HEAD

git -C lynx-oss grep -n \
  'RTS/NativeContext is not available in OSS build' -- \
  core/runtime/mts_context_factory.cc

git -C lynx-oss ls-files \
  'core/runtime/rts/**' \
  'platform/android/lynx_clay/**' \
  '*LynxUIRendererClay*'

git -C lynx-oss grep -n 'class StaticPageHost' -- \
  platform/android/lynx_android/src/main/java/com/lynx/tasm/StaticPageHost.java

git -C lynx-oss grep -n 'interface IUIRendererCreator' -- \
  platform/android/lynx_android/src/main/java/com/lynx/tasm/IUIRendererCreator.java

git -C lynx-oss grep -n 'testonly = true' -- clay/BUILD.gn
git -C lynx-oss status --short --branch
git -C lynx-oss diff --exit-code
```

At the audited revision, the `ls-files` command for the RTS implementation,
Android Clay directory, and Android Clay renderer class produces no output.

The automated source-boundary probe used by this audit was equivalent to:

```bash
set -euo pipefail
test "$(git rev-parse HEAD)" = \
  "0df14207cebb060f1bed8de12b64a1119dee8f06"
test -f platform/android/lynx_android/src/main/java/com/lynx/tasm/StaticPageHost.java
test -f platform/android/lynx_android/src/main/java/com/lynx/tasm/StaticPageInstance.java
test -f platform/embedder/public/capi/lynx_windowless_renderer_capi.h
test ! -d core/runtime/rts
test ! -d platform/android/lynx_clay
test -z "$(git ls-files '*LynxUIRendererClay*')"
git grep -q 'RTS/NativeContext is not available in OSS build' -- \
  core/runtime/mts_context_factory.cc
git grep -q 'return new LynxUIRenderer();' -- \
  platform/android/lynx_android/src/main/java/com/lynx/tasm/behavior/LynxUIRendererCreator.java
git grep -q 'testonly = true' -- clay/BUILD.gn
```

This probe passed. The clean OSS checkout also passed `git diff --exit-code` and
`git diff --cached --exit-code`.

No Lynx build or Android device test was claimed. There is no patch to compile,
and the stock runtime fails at the missing RTS Native context/bundle before the
Yew application can be loaded. Patch-apply verification is therefore not
applicable for this audit.

## Current blockers

1. No public renderer C ABI suitable for Rust/Yew element reconciliation.
2. No OSS RTS Native context or context bundle implementation.
3. No public application descriptor and factory registration contract.
4. No Android bridge from native runtime lifecycle to the public static-page
   host.
5. No Android Clay renderer/shell/JNI/build implementation in the OSS tree.
6. No end-to-end public Android test proving a native static page on either the
   standard or Clay renderer.
7. Separate Yew patch provenance, reproducibility, and upstream compatibility
   still need their own public review.

Until blockers 1 through 4 are resolved, a native Yew static page cannot run on
stock OSS Android Lynx. Until blocker 5 is also resolved, it cannot claim Android
Clay compatibility.
