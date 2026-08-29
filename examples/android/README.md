# Standalone Android example

This Kotlin app hosts the Yew or Dioxus counter natively, or either framework as
a guest in statically embedded WAMR, through Lynx's native renderer function table. It
registers one native host per `LynxView`, links one backend-specific Rust arm64
archive, and does not render a template bundle or enable JS/MTS execution.

## Build

From the repository root:

```bash
./scripts/build-android.sh --backend yew
./scripts/build-android.sh --backend dioxus
./scripts/build-android.sh --backend wasm-dioxus
./scripts/build-android.sh --backend wasm-yew
```

`--clean` discards generated integration outputs. After a matching online build,
`--offline` constrains Cargo, Gradle, and the Lynx source build to prepared local
inputs. Backend-specific staging prevents cross-framework cache reuse.

The app opts into
`org.lynxsdk.lynx:lynx-native-renderer:0.0.1-0df14207` and receives
`liblynx_native_renderer.so`. It does not depend on the separately built stock
`org.lynxsdk.lynx:lynx:0.0.1-0df14207` / `liblynx.so` product, whose behavior is
preserved for other consumers.

Requirements:

- JDK 11
- Android SDK platform 33 and build-tools 33.0.1
- Android NDK 21.1.6352462 for Lynx AARs
- Android NDK 25.2.9519653 for Rust/JNI linking
- CMake 3.22.1
- Rust 1.85.0 with `aarch64-linux-android` and `wasm32-wasip1`
- Node.js 22.18.0 for Lynx's source build tooling

Android builds are supported on Linux and macOS. The build selects an installed
`linux-x86_64`, `darwin-arm64`, or `darwin-x86_64` NDK prebuilt host directory.
Set `ANDROID_NDK_HOST_TAG` (for example, `darwin-x86_64`) to override automatic
selection when using a translated or nonstandard NDK installation.

The app supports only `arm64-v8a`. APKs are written to
`.deps/android/apks/lynx-element-bridge-{yew,dioxus,wasm-dioxus,wasm-yew}.apk`.
The `wasm-dioxus` and `wasm-yew` APKs open a URL launcher. It accepts HTTP and
HTTPS URLs for compatible Lynx Element Bridge `wasm32-wasip1` modules, records
the 20 most recently confirmed URLs, and opens each downloaded module in a new
activity. Downloads are limited to 16 MiB, a 15-second connection timeout, a
30-second read timeout, and five redirects. History stores complete URLs in
unencrypted application preferences, including query parameters; do not use
credential-bearing URLs on a shared device.

For local iteration, serve a guest and forward the device port:

```bash
python3 -m http.server 8000 --directory target/wasm-guests/wasm-yew/initial/wasm32-wasip1/release
adb reverse tcp:8000 tcp:8000
```

Then enter `http://127.0.0.1:8000/yew_lynx_counter.wasm`. The URL launcher is a
development tool: downloaded modules must implement this repository's guest ABI
and protocol, and should come from a trusted source. WAMR isolates guest memory,
but the app does not currently enforce a guest CPU execution timeout.

The Wasm modes package initial and replacement variants of
`assets/dioxus_counter.wasm` or `assets/yew_counter.wasm`, statically link WAMR into
`liblynx_element_bridge.so`, and exposes `MainActivity.replaceWasmModule()` as a
local replacement test hook. Both variants compile the same framework crate and
DSL source; `replacement-fixture` changes the candidate's initial count to
`Count: 100`. The build rejects any packaged `.lynx.bundle`;
stock `liblynx.so`; Quick/PrimJS, NAPI, standalone Wasm runtime, or V8 shared
libraries; and LynxJSSDK's `assets/lynx_core.js`. It also verifies the native
product and app dependency graphs, the selected backend marker, forbidden ELF
`DT_NEEDED`/undefined runtime symbols, and the exported C API. PrimJS lock and
preparation are used only to reproducibly build/test the preserved stock
artifact, not as native product or app dependencies.

## Acceptance

Do not rebuild the APK when validating a previously recorded artifact. Run the
acceptance script with its backend, serial, APK, and evidence directory:

```bash
python3 scripts/android-device-acceptance.py \
  --backend dioxus \
  --serial "$ANDROID_SERIAL" \
  --apk .deps/android/apks/lynx-element-bridge-dioxus.apk \
  --evidence-dir .deps/android/device-acceptance-binary-native-dioxus-20260822-0015-final
```

Use the corresponding `wasm-dioxus` or `wasm-yew` backend and APK for a Wasm
run. After tapping to `Count: 1`, the script sends an explicit acceptance intent
to the foreground activity, which reads the packaged replacement asset and calls
`replaceWasmModule()`. A passing run must visibly show the candidate's
`Count: 100`, distinct from both `Count: 0` and `Count: 1`, and log the completed
replacement; no network, second business crate, or state migration is involved.

The script verifies tap update, rotation recreation, force-stop/reopen,
repeated mount/tap/destroy cycles, backend identity, native/Wasm
diagnostics, APK contents, and fresh/post-interaction process maps. It restores
rotation settings and does not persist the ADB serial.

The native acceptance flow writes these eight evidence files:

```text
fresh-count-0.png
after-tap-count-1.png
after-activity-recreation.png
after-force-stop-reopen.png
logcat.txt
maps-fresh.txt
maps-after-interaction.txt
summary.json
```

Wasm runs additionally write `after-wasm-replace-count-100.png` and report
`wasm_replacement_expected_count=100` and
`wasm_replacement_candidate_detected=true` in `summary.json`.

Final patch-0015 binary-native evidence from 2026-08-22 was recorded on an
anonymous Xiaomi Redmi K60 Pro physical device running Android 13/API 33,
arm64-v8a:

| Backend | APK SHA-256 | Evidence | `onCreate` | `onDestroy` | diagnostics/backend |
| --- | --- | --- | ---: | ---: | ---: |
| Yew | `121c20a6bc82d1570eb24cfb37c84a5d82c414bc1b176c4b40ac8294fe45903d` | `.deps/android/device-acceptance-binary-native-yew-20260822-0015-final` | 9 | 4 | 9 |
| Dioxus | `008d64415874bff997a47c1afcb25dc2e87c5fe4e6f513acaad2bf8273f3afdc` | `.deps/android/device-acceptance-binary-native-dioxus-20260822-0015-final` | 9 | 4 | 9 |

Both runs passed fresh launch, tap, recreation, force-stop/reopen, and
three cycles, with every functional result flag true and zero crash markers.
Both recorded `renderer_mode=native`, `bts_runtime=false`, `mts_context=false`,
and `template=false`. `proc_maps_checked` is true: the five required libraries
were mapped at fresh launch and after interaction, the forbidden set was empty,
and Quick/NAPI/Wasm/V8 mapping flags were false.
