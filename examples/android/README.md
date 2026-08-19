# Standalone Android example

This Kotlin application is the public stock OSS Lynx host for the Yew counter.
It consumes locally published AARs built from the pinned `third_party/lynx`
submodule, enables synchronous MTS modules, registers one `YewLynxModule` per
`LynxView`, links the real arm64 Rust archive through JNI, and loads the ordinary
LepusNG bundle from application assets.

From the repository root, use the single orchestration command:

```bash
./scripts/build-android.sh
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

The only supported application ABI is `arm64-v8a`. Generated Maven artifacts,
native libraries, bundles, and APKs remain ignored. The final APK is written to
`app/build/outputs/apk/debug/app-debug.apk`.

After connecting an ARM64 physical device through ADB, run the lifecycle
acceptance flow from the repository root:

```bash
python3 scripts/android-device-acceptance.py \
  --serial "$ANDROID_SERIAL" \
  --apk examples/android/app/build/outputs/apk/debug/app-debug.apk \
  --evidence-dir .deps/android/device-evidence
```

The script locates and taps the rendered Increment control from raw screenshots,
checks that the visible counter region changes, recreates the Activity through
rotation, force-stops and reopens the process, and repeats mount/tap/destroy
cycles. It saves PNGs for visual review of the exact `Count: 0` and `Count: 1`
states, restores the original rotation settings, and does not write the ADB
serial into evidence. Physical-device access and credentials are intentionally
outside public CI.
