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
packaged `.lynx.bundle` and verifies that exactly the selected backend marker is
present.

## Acceptance

Do not rebuild the APK when validating a previously recorded artifact. Run the
acceptance script with its backend, serial, APK, and evidence directory:

```bash
python3 scripts/android-device-acceptance.py \
  --backend dioxus \
  --serial "$ANDROID_SERIAL" \
  --apk .deps/android/apks/lynx-element-bridge-dioxus.apk \
  --evidence-dir .deps/android/device-acceptance-native-dioxus-20260822-release-safe-success
```

The script verifies timer-only update, tap update, rotation recreation,
force-stop/reopen, repeated mount/tap/destroy cycles, backend identity, and the
native diagnostics line. It restores rotation settings and does not persist the
ADB serial.

Final patch-0009 release-safe evidence from 2026-08-22 was recorded on an
anonymous physical OPPO PGBM10 device running Android 13/API 33, arm64-v8a:

| Backend | APK SHA-256 | Evidence | `onCreate` | `onDestroy` | diagnostics/backend |
| --- | --- | --- | ---: | ---: | ---: |
| Yew | `685e8000ac037607fc9cd870d0445293c3f11ec11d6c00b2a375033ed468a1bf` | `.deps/android/device-acceptance-native-yew-20260822-release-safe-success` | 9 | 4 | 9 |
| Dioxus | `6a51be912a01764566edbf8bea859effe0af073116a785fd6251d71f53664513` | `.deps/android/device-acceptance-native-dioxus-20260822-release-safe-success` | 10 | 5 | 10 |

Both runs passed fresh launch, timer, tap, recreation, force-stop/reopen, and
three cycles, with all six acceptance result flags true. Both recorded
`renderer_mode=native`, `bts_runtime=false`, `mts_context=false`, and
`template=false`, with zero wrong-backend, crash, or timer-teardown markers.
Dioxus totals are one higher because the OS performed one extra recreation.

The stock Lynx AAR still packages and loads Quick, PrimJS, and NAPI;
binary-native packaging remains a blocked follow-up milestone, and complete
JS-engine removal is not claimed.
