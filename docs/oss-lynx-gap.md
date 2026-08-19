# Stock OSS Lynx audit for the MTS/Fiber route

## Corrected conclusion

The official public
[`lynx-family/lynx`](https://github.com/lynx-family/lynx) repository was audited
at commit
[`0df14207cebb060f1bed8de12b64a1119dee8f06`](https://github.com/lynx-family/lynx/tree/0df14207cebb060f1bed8de12b64a1119dee8f06).

Stock OSS Lynx at this revision contains the ordinary APIs needed by this
repository's chosen design:

```text
public Android LynxModule
  -> synchronous MTS module proxy
  -> public Fiber Element globals
  -> ordinary context-type 1 LepusNG template
  -> stock renderer
```

The repository implements that path with the exact Lynx revision pinned as a
source submodule and no Lynx patch. It now includes a standalone Android host,
local stock AAR build, real Rust/JNI link, arm64 APK, and one physical-device
acceptance run.

## Stock implementation evidence

- The ordinary `LepusNGContextType` has a stock context and context bundle in
  [`mts_context_factory.cc`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/runtime/mts_context_factory.cc).
- Public typed Fiber globals, including page/element creation, mutation,
  listeners, unique IDs, and flush, are declared in
  [`element-api.d.ts`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/js_libraries/type-element-api/types/element-api.d.ts).
- The ordinary LepusNG renderer registers Fiber operations and `lynx.module()`
  in
  [`renderer_ng.cc`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/runtime/lepusng/bindings/renderer_ng.cc).
- Module lookup creates a stock native-module proxy in
  [`lynx_lepus_module_manager.cc`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/core/runtime/lepus/bindings/modules/lynx_lepus_module_manager.cc).
- Android exposes public `LynxModule` construction and destruction in
  [`LynxModule.java`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/jsbridge/LynxModule.java),
  module registration and the MTS opt-in in
  [`LynxBaseConfigurator.java`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/tasm/group/LynxBaseConfigurator.java),
  and synchronous `String` method signatures through
  [`LynxMethodWrapper.java`](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/platform/android/lynx_android/src/main/java/com/lynx/jsbridge/LynxMethodWrapper.java).

The Fiber globals are a declared public typed surface. `lynx.module()` is
present in the pinned stock implementation, but is absent from that revision's
declared public
[`Lynx` TypeScript interface](https://github.com/lynx-family/lynx/blob/0df14207cebb060f1bed8de12b64a1119dee8f06/js_libraries/types/types/main-thread/lynx.d.ts).
This project therefore pins the implementation revision and does not describe
`module()` as a stable TypeScript interface.

## Why the earlier blockers do not apply

An earlier proposal tried to run Yew through a direct RTS Native runtime and an
Android Clay renderer. That discarded direct-native route is genuinely blocked
in stock OSS Lynx: the pinned factory returns no RTS or RTS Native context or
bundle, and the public tree does not provide the proposed Android Clay host.

Those facts do not block the implemented MTS/Fiber route. The current template
is ordinary LepusNG, not an RTS Native descriptor. Rust does not mutate Lynx C++
objects and JNI does not call hidden Lynx symbols. Instead, Rust returns a
strict mutation envelope through a public Android module, and ordinary MTS code
applies it through public Fiber globals before the stock renderer consumes the
tree.

Direct JNI binding to hidden stock Lynx C++ symbols remains unsupported and is
a no-go for this project. The old RTS Native/Clay findings remain only as the
historical explanation for rejecting that route; they are not current blockers.

## Repository evidence and closed integration gap

The template build uses exact npm development dependencies `esbuild` 0.25.9
and `@lynx-js/tasm` 0.0.51. It emits ignored shell, encoder-input, and bundle
artifacts, then verifies both the native codec path and a forced WebAssembly
path by decoding the bundle and requiring `context-type` 1 and
`is-lepusng-binary: true`.

Rust tests validate protocol v1 and the counter C ABI. MTS tests validate the
broker against mock Fiber globals. Android tests compile the public-module
adapter and real JNI source against stock-API stubs and a mock Rust ABI. A C
smoke test links the real host Rust archive, and CI builds the Android arm64
archive. Patched Yew tests validate the focused native renderer and macro
behavior.

`examples/android` closes the audited integration gap: it enables MTS,
registers one non-shared `YewLynxModule` per `LynxView`, packages the Rust
archive through the real JNI shared library, and loads the ordinary bundle via
the stock template API. The build publishes six AARs from the pinned submodule,
assembles online and offline, and rejects non-arm64 APK contents. One Android 15
ARM64 physical-device run verified initial render, tap update, recreation,
process reopen, and repeated teardown. This remains evidence for the exact pin
and device only, not a general performance or compatibility certification.
