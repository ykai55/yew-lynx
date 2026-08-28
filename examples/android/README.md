# Standalone Android example

This Kotlin app hosts either the Yew or Dioxus counter through Lynx's native
renderer function table. It registers one native host per `LynxView`, links one
backend-specific Rust arm64 archive, and does not render a template bundle or
enable JS/MTS application execution.

## Build

From the repository root:

```bash
./scripts/build-android.sh --backend yew
./scripts/build-android.sh --backend dioxus
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
- Rust 1.85.0 with `aarch64-linux-android`
- Node.js 22.18.0 for Lynx's source build tooling

The app supports only `arm64-v8a`. APKs are written to
`.deps/android/apks/lynx-element-bridge-{yew,dioxus}.apk`. The build rejects any
packaged `.lynx.bundle`; stock `liblynx.so`; Quick/PrimJS, NAPI, Wasm, or V8
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

The script verifies timer-only update, tap update, rotation recreation,
force-stop/reopen, repeated mount/tap/destroy cycles, backend identity, native
diagnostics, APK contents, and fresh/post-interaction process maps. It restores
rotation settings and does not persist the ADB serial.

Each successful evidence directory has exactly these nine files:

```text
fresh-count-0.png
after-timer-fired.png
after-tap-count-1.png
after-activity-recreation.png
after-force-stop-reopen.png
logcat.txt
maps-fresh.txt
maps-after-interaction.txt
summary.json
```

Final patch-0015 binary-native evidence from 2026-08-22 was recorded on an
anonymous Xiaomi Redmi K60 Pro physical device running Android 13/API 33,
arm64-v8a:

| Backend | APK SHA-256 | Evidence | `onCreate` | `onDestroy` | diagnostics/backend |
| --- | --- | --- | ---: | ---: | ---: |
| Yew | `121c20a6bc82d1570eb24cfb37c84a5d82c414bc1b176c4b40ac8294fe45903d` | `.deps/android/device-acceptance-binary-native-yew-20260822-0015-final` | 9 | 4 | 9 |
| Dioxus | `008d64415874bff997a47c1afcb25dc2e87c5fe4e6f513acaad2bf8273f3afdc` | `.deps/android/device-acceptance-binary-native-dioxus-20260822-0015-final` | 9 | 4 | 9 |

Both runs passed fresh launch, timer, tap, recreation, force-stop/reopen, and
three cycles, with every functional result flag true and zero crash markers.
Both recorded `renderer_mode=native`, `bts_runtime=false`, `mts_context=false`,
and `template=false`. `proc_maps_checked` is true: the five required libraries
were mapped at fresh launch and after interaction, the forbidden set was empty,
and Quick/NAPI/Wasm/V8 mapping flags were false.
