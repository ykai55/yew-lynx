# Standalone Android example

This app starts with a runtime picker. Native opens the Yew or Dioxus counter
selected by the Android Studio build variant. WASM opens a URL launcher for an
externally built compatible guest. Both paths use Lynx's native renderer function
table and do not enable JS/MTS or render a template bundle.

The Gradle project has three modules:

- `app`: runtime picker, Native/WASM activities, URL history, and download cache.
- `bridge-native`: builds the selected Yew or Dioxus Rust static library and
  packages `liblynx_element_bridge_native.so`.
- `bridge-wamr`: builds the Rust WAMR host and packages
  `liblynx_element_bridge_wamr.so`. It never builds or packages a guest `.wasm`.

## Script Build

From the repository root:

```bash
./scripts/build-android.sh --backend yew
./scripts/build-android.sh --backend dioxus
```

`--clean` discards generated integration outputs. After a matching online build,
`--offline` constrains Cargo, Gradle, and the Lynx source build to prepared local
inputs. Every APK contains the selected Native runtime and the framework-neutral
WAMR host.

## Android Studio

The IDE build expects initialized submodules, the patched Yew checkout, and the
locally published patched Lynx Maven artifacts. Prepare those once:

```bash
./scripts/build-android.sh --prepare-only
```

Then open `examples/android` in Android Studio, select `yewDebug` or
`dioxusDebug` in **Build Variants**, and use the normal Build/Run actions. Gradle
directly drives Cargo, archive staging, CMake, and APK packaging; it does not call
the top-level build script. Rust changes in the selected Native backend or WAMR
host are included in an IDE incremental build.

Requirements:

- JDK 11
- Android SDK platform 33 and build-tools 33.0.1
- Android NDK 21.1.6352462 for preparing Lynx AARs
- Android NDK 25.2.9519653 for Rust/JNI linking
- CMake 3.22.1
- Rust 1.85.0 with `aarch64-linux-android`
- Node.js 22.18.0 for preparing Lynx AARs

Android builds are supported on Linux and macOS. Gradle resolves the SDK through
AGP, so the standard `examples/android/local.properties` setting works:

```properties
sdk.dir=/path/to/Android/sdk
cargo.path=/absolute/path/to/cargo
```

`cargo.path` is optional. Gradle resolves Cargo in this order: the explicit
`cargo.path`, the Android Studio or shell `PATH`, then `~/.cargo/bin/cargo`.

Set `ANDROID_NDK_HOST_TAG` (for example, `darwin-x86_64`) only for a translated
or nonstandard NDK installation. The top-level shell build still requires
`ANDROID_HOME` or `ANDROID_SDK_ROOT` because it invokes SDK tools before Gradle.

The app supports only `arm64-v8a`. Script-built APKs are written to
`.deps/android/apks/lynx-element-bridge-{yew,dioxus}.apk`.

## External WASM

The WASM entry accepts HTTP and HTTPS URLs for compatible Lynx Element Bridge
`wasm32-wasip1` modules. It records the 20 most recently confirmed URLs and opens
each downloaded module in a new activity. Downloads are limited to 16 MiB, a
15-second connection timeout, a 30-second read timeout, and five redirects.
History stores complete URLs in unencrypted application preferences, including
query parameters; do not use credential-bearing URLs on a shared device.

The guest is built outside the Android project. For local iteration, serve its
output and forward the device port:

```bash
python3 -m http.server 8000 --directory /path/to/external/guest/output
adb reverse tcp:8000 tcp:8000
```

Then enter a URL such as `http://127.0.0.1:8000/page.wasm`. Downloaded modules
must implement this repository's guest ABI and protocol and should come from a
trusted source. WAMR isolates guest memory, but the app does not enforce a guest
CPU execution timeout.

The APK contains no guest `.wasm`. The build also rejects `.lynx.bundle`, stock
`liblynx.so`, Quick/PrimJS, NAPI, standalone Wasm runtime, V8 shared libraries,
and LynxJSSDK's `assets/lynx_core.js`.

## Native Acceptance

The existing device acceptance script validates the Native entry:

```bash
python3 scripts/android-device-acceptance.py \
  --backend yew \
  --serial "$ANDROID_SERIAL" \
  --apk .deps/android/apks/lynx-element-bridge-yew.apk \
  --evidence-dir .deps/android/device-acceptance-native-yew
```

The URL download flow currently relies on build tests and requires separate
device interaction coverage.
