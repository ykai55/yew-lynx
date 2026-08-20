# Standalone Android example

This Kotlin application is the public stock OSS Lynx host for either counter
backend. It consumes locally published AARs built from the pinned
`third_party/lynx` submodule, registers one framework-neutral
`LynxElementBridgeModule` per `LynxView`, links exactly one real arm64 Rust
archive through the shared JNI library, and loads the ordinary LepusNG bundle.

From the repository root, use the orchestration command for the desired backend:

```bash
./scripts/build-android.sh
./scripts/build-android.sh --backend dioxus
```

Use `--clean` to discard generated integration outputs before rebuilding. The
default cached mode fingerprints the Lynx/Yew pins, Habitat and PrimJS locks,
Rust/Node lockfiles, and Android build inputs before reusing outputs. Once a
successful online build has prepared a matching cache, `--offline` constrains
Gradle, npm, and Cargo to local inputs.

Requirements:

- JDK 11
- Android SDK platform 33 and build-tools 33.0.1
- Android NDK 21.1.6352462 for the pinned Lynx AAR build
- Android NDK 25.2.9519653 for the Rust/JNI shared library link
- CMake 3.22.1
- Rust 1.85.0 with `aarch64-linux-android`
- Node.js 22.18.0 and npm

The default backend is Yew. Backend-specific Rust staging, AGP/CMake staging,
APKs, and evidence prevent one framework from reusing the other's native cache.
The only supported application ABI is `arm64-v8a`. Final APKs are written to
`.deps/android/apks/lynx-element-bridge-yew.apk` and
`.deps/android/apks/lynx-element-bridge-dioxus.apk`; matching build evidence is
written under `.deps/android/`.

After connecting an ARM64 physical device through ADB, run the lifecycle
acceptance flow from the repository root:

```bash
python3 scripts/android-device-acceptance.py \
  --backend dioxus \
  --serial "$ANDROID_SERIAL" \
  --apk .deps/android/apks/lynx-element-bridge-dioxus.apk \
  --evidence-dir .deps/android/device-evidence-dioxus
```

The script locates and taps the rendered Increment control from raw screenshots,
checks that the visible counter region changes, recreates the Activity through
rotation, force-stops and reopens the process, and repeats mount/tap/destroy
cycles. It saves PNGs for visual review of the exact `Count: 0` and `Count: 1`
states, requires the expected backend identity from the linked Rust archive in
logcat, restores the original rotation settings, and does not write the ADB
serial into evidence. On 2026-08-20, both Yew and Dioxus independently passed
this flow on Android 15/API 35 arm64 physical devices; exact device and APK
identities are recorded in `COMPATIBILITY.md`. Physical-device access remains
outside public CI.
