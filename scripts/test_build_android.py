#!/usr/bin/env python3

import pathlib
import re
import subprocess
import unittest


ROOT = pathlib.Path(__file__).resolve().parent.parent
BUILD_SCRIPT = (ROOT / "scripts" / "build-android.sh").read_text()
VERIFY_SCRIPT = (ROOT / "scripts" / "verify.sh").read_text()
BUILD_UTILS = (ROOT / "scripts" / "android-build-utils.sh").read_text()
BOOTSTRAP_YEW = (ROOT / "scripts" / "bootstrap-yew.sh").read_text()
ANDROID_APP_GRADLE = (ROOT / "examples" / "android" / "app" / "build.gradle.kts").read_text()
ANDROID_ROOT_GRADLE = (ROOT / "examples" / "android" / "build.gradle.kts").read_text()
ANDROID_NATIVE_GRADLE = (
    ROOT / "examples" / "android" / "bridge-native" / "build.gradle.kts"
).read_text()
ANDROID_WAMR_GRADLE = (
    ROOT / "examples" / "android" / "bridge-wamr" / "build.gradle.kts"
).read_text()
ANDROID_SETTINGS = (ROOT / "examples" / "android" / "settings.gradle.kts").read_text()
ANDROID_MANIFEST = (
    ROOT / "examples" / "android" / "app" / "src" / "main" / "AndroidManifest.xml"
).read_text()
ANDROID_MAIN_ACTIVITY = (
    ROOT
    / "examples"
    / "android"
    / "app"
    / "src"
    / "main"
    / "java"
    / "com"
    / "yew"
    / "lynx"
    / "example"
    / "MainActivity.kt"
).read_text()
WASM_URL_ACTIVITY = (
    ROOT
    / "examples"
    / "android"
    / "app"
    / "src"
    / "main"
    / "java"
    / "com"
    / "yew"
    / "lynx"
    / "example"
    / "WasmUrlActivity.kt"
).read_text()
ANDROID_LAUNCHER_ACTIVITY = (
    ROOT / "examples/android/app/src/main/java/com/yew/lynx/example/LauncherActivity.kt"
).read_text()
ANDROID_WASM_ACTIVITY = (
    ROOT / "examples/android/app/src/main/java/com/yew/lynx/example/WasmActivity.kt"
).read_text()
WASM_MODULE_FILE = (
    ROOT
    / "examples"
    / "android"
    / "app"
    / "src"
    / "main"
    / "java"
    / "com"
    / "yew"
    / "lynx"
    / "example"
    / "WasmModuleFile.kt"
).read_text()
ANDROID_CMAKE = (ROOT / "adapters" / "android" / "CMakeLists.txt").read_text()
ANDROID_JNI = (
    ROOT / "adapters" / "android" / "src" / "main" / "cpp" / "lynx_element_bridge_jni.cc"
).read_text()
WAMR_BUILD_RS = (
    ROOT / "crates" / "element-bridge-wamr-host" / "build.rs"
).read_text()
PUBLIC_TOOLS_SHARED_SHA = "ff47fee7d41ee3e8e8561041b1ce2c8b50e923ea"
WAMR_SHA = "25bd7eb63e828e4bd242cc9b38d260b4b31c6605"


class BuildAndroidStaticTest(unittest.TestCase):
    def test_wasm_url_launcher_is_scoped_and_bounded(self):
        self.assertIn('android.permission.INTERNET', ANDROID_MANIFEST)
        self.assertIn('android:name=".LauncherActivity"', ANDROID_MANIFEST)
        self.assertIn('android:usesCleartextTraffic="true"', ANDROID_MANIFEST)
        self.assertIn('const val MAX_BYTES = 16 * 1024 * 1024', WASM_MODULE_FILE)
        self.assertIn('const val CONNECT_TIMEOUT_MS = 15_000', WASM_URL_ACTIVITY)
        self.assertIn('const val READ_TIMEOUT_MS = 30_000', WASM_URL_ACTIVITY)
        self.assertIn('const val MAX_REDIRECTS = 5', WASM_URL_ACTIVITY)
        self.assertIn('url.protocol == "http" || url.protocol == "https"', WASM_URL_ACTIVITY)
        self.assertIn('activeConnection?.disconnect()', WASM_URL_ACTIVITY)
        self.assertIn('Thread.currentThread().isInterrupted', WASM_URL_ACTIVITY)

    def test_downloaded_wasm_uses_private_cache_and_a_new_activity(self):
        self.assertIn('fileNamePattern.matches(fileName)', WASM_MODULE_FILE)
        self.assertIn('context.cacheDir.canonicalFile', WASM_MODULE_FILE)
        self.assertIn('java.io.File.createTempFile(', WASM_URL_ACTIVITY)
        self.assertIn('Intent(this, WasmActivity::class.java)', WASM_URL_ACTIVITY)
        self.assertIn('result.getOrNull()?.delete()', WASM_URL_ACTIVITY)
        self.assertIn('file.delete()', WASM_URL_ACTIVITY)
        self.assertIn('WasmModuleFile.read(this, fileName)', ANDROID_WASM_ACTIVITY)
        self.assertNotIn('assets.open(', ANDROID_WASM_ACTIVITY)

    def test_wasm_url_history_records_confirmed_urls(self):
        self.assertLess(WASM_URL_ACTIVITY.index('recordHistory(value)'), WASM_URL_ACTIVITY.index('validateUrl(URL(value))'))
        self.assertIn('const val HISTORY_LIMIT = 20', WASM_URL_ACTIVITY)
        self.assertIn('JSONArray(history).toString()', WASM_URL_ACTIVITY)

    def test_ndk_tools_use_dynamic_host_prebuilt_directory(self):
        gradle_files = (
            ROOT / "examples" / "android" / "bridge-native" / "build.gradle.kts",
            ROOT / "examples" / "android" / "bridge-wamr" / "build.gradle.kts",
        )

        self.assertIn("resolve_android_ndk_prebuilt_dir", BUILD_SCRIPT)
        self.assertIn("resolve_android_ndk_prebuilt_dir", VERIFY_SCRIPT)
        self.assertIn("ANDROID_NDK_HOST_TAG", BUILD_UTILS)
        self.assertIn("darwin-arm64 darwin-x86_64", BUILD_UTILS)
        self.assertNotIn("prebuilt/linux-x86_64", BUILD_SCRIPT)
        self.assertNotIn("prebuilt/linux-x86_64", VERIFY_SCRIPT)
        for gradle_file in gradle_files:
            gradle = gradle_file.read_text()
            self.assertIn('System.getenv("ANDROID_NDK_HOST_TAG")', gradle)
            self.assertIn("androidComponents.sdkComponents.sdkDirectory", gradle)
            self.assertIn('listOf("darwin-arm64", "darwin-x86_64")', gradle)
            self.assertIn('listOf("linux-x86_64")', gradle)
            self.assertNotIn('System.getenv("ANDROID_HOME")', gradle)
            self.assertNotIn('System.getenv("ANDROID_SDK_ROOT")', gradle)
            self.assertNotIn("prebuilt/linux-x86_64", gradle)

    def test_android_gradle_supports_local_cargo_path(self):
        self.assertIn('rootDir.resolve("local.properties")', ANDROID_ROOT_GRADLE)
        self.assertIn('getProperty("cargo.path")', ANDROID_ROOT_GRADLE)
        self.assertIn('System.getenv("PATH")', ANDROID_ROOT_GRADLE)
        self.assertIn('File(it, "cargo")', ANDROID_ROOT_GRADLE)
        self.assertIn('".cargo/bin/cargo"', ANDROID_ROOT_GRADLE)
        self.assertLess(
            ANDROID_ROOT_GRADLE.index('getProperty("cargo.path")'),
            ANDROID_ROOT_GRADLE.index('System.getenv("PATH")'),
        )
        self.assertLess(
            ANDROID_ROOT_GRADLE.index('System.getenv("PATH")'),
            ANDROID_ROOT_GRADLE.index('".cargo/bin/cargo"'),
        )
        self.assertIn('extra["cargoExecutable"]', ANDROID_ROOT_GRADLE)
        for gradle in (ANDROID_NATIVE_GRADLE, ANDROID_WAMR_GRADLE):
            self.assertIn('val cargoExecutable: String by rootProject.extra', gradle)
            self.assertIn('mutableListOf(cargoExecutable, "build", "--locked")', gradle)

    def test_ndk_host_selection_behavior(self):
        result = subprocess.run(
            [ROOT / "scripts" / "test-android-build-utils.sh"],
            check=False,
            capture_output=True,
            text=True,
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Android build utility tests passed", result.stdout)

    def test_first_party_android_scripts_use_portable_sha256(self):
        for script in (
            BUILD_SCRIPT,
            (ROOT / "scripts" / "prepare-hab.sh").read_text(),
            (ROOT / "scripts" / "prepare-primjs.sh").read_text(),
        ):
            self.assertIn("sha256_checksum", script)
            self.assertNotIn("sha256sum ", script)
        self.assertIn("shasum -a 256", BUILD_UTILS)

    def test_macos_compatible_tools_are_used_throughout_the_build(self):
        self.assertIn("scripts/android-build-utils.sh", BUILD_SCRIPT)
        self.assertIn('"$ndk21_llvm_bin/llvm-strings"', BUILD_SCRIPT)
        self.assertNotIn("| strings)", BUILD_SCRIPT)
        self.assertNotIn("mapfile", BOOTSTRAP_YEW)

    def test_wamr_bound_checks_are_target_specific(self):
        self.assertIn(
            "(disable_hw_bound_check, disable_stack_hw_bound_check)", WAMR_BUILD_RS
        )
        self.assertIn('"linux" => {', WAMR_BUILD_RS)
        self.assertIn('("0", "0")', WAMR_BUILD_RS)
        self.assertIn('"android" => {', WAMR_BUILD_RS)
        self.assertIn('("0", "1")', WAMR_BUILD_RS)
        self.assertEqual(WAMR_BUILD_RS.count('define("WASM_DISABLE_HW_BOUND_CHECK"'), 1)
        self.assertEqual(
            WAMR_BUILD_RS.count('"WASM_DISABLE_STACK_HW_BOUND_CHECK"'), 1
        )
        self.assertIn("Android manages the main thread stack guard", WAMR_BUILD_RS)

    def test_wamr_runtime_is_static_pinned_and_guest_is_external(self):
        self.assertIn(f'WAMR_SHA="{WAMR_SHA}"', BUILD_SCRIPT)
        self.assertIn(f'WAMR_SHA="{WAMR_SHA}"', VERIFY_SCRIPT)
        self.assertIn("liblynx_element_bridge_wamr.so", BUILD_SCRIPT)
        self.assertIn("wasm_guest_sha256=external-url-only", BUILD_SCRIPT)
        self.assertNotIn("dioxus_counter.wasm", BUILD_SCRIPT)
        self.assertNotIn("yew_counter.wasm", BUILD_SCRIPT)
        self.assertIn("libwasm\\.so", BUILD_SCRIPT)

    def test_android_modules_split_native_and_wamr_runtimes(self):
        self.assertIn('include(":bridge-native")', ANDROID_SETTINGS)
        self.assertIn('include(":bridge-wamr")', ANDROID_SETTINGS)
        self.assertIn('implementation(project(":bridge-native"))', ANDROID_APP_GRADLE)
        self.assertIn('implementation(project(":bridge-wamr"))', ANDROID_APP_GRADLE)
        self.assertIn('create("yew")', ANDROID_NATIVE_GRADLE)
        self.assertIn('create("dioxus")', ANDROID_NATIVE_GRADLE)
        self.assertIn("lynx_element_bridge_native", ANDROID_NATIVE_GRADLE)
        self.assertIn("lynx_element_bridge_wamr", ANDROID_WAMR_GRADLE)
        self.assertNotIn("wasm32-wasip1", ANDROID_APP_GRADLE + ANDROID_WAMR_GRADLE)
        self.assertIn("MainActivity::class.java", ANDROID_LAUNCHER_ACTIVITY)
        self.assertIn("WasmUrlActivity::class.java", ANDROID_LAUNCHER_ACTIVITY)

    def test_android_ci_covers_native_and_wasm_backends(self):
        workflow = (ROOT / ".github" / "workflows" / "android-integration.yml").read_text()

        self.assertIn("backend: [yew, dioxus]", workflow)
        self.assertIn('./scripts/build-android.sh --backend "${{ matrix.backend }}" --clean', workflow)

    def test_tools_shared_pin_is_publicly_reachable_revision(self):
        pin_patch = ROOT / "patches/lynx/0016-Pin-public-tools-shared-revision.patch"

        self.assertIn(f'LYNX_TOOLS_SHARED_SHA="{PUBLIC_TOOLS_SHARED_SHA}"', BUILD_SCRIPT)
        self.assertIn(f'LYNX_TOOLS_SHARED_SHA="{PUBLIC_TOOLS_SHARED_SHA}"', VERIFY_SCRIPT)
        self.assertTrue(pin_patch.is_file())
        pin_patch_text = pin_patch.read_text()
        self.assertIn("diff --git a/dependencies/DEPS b/dependencies/DEPS", pin_patch_text)
        self.assertNotIn("dependencies/DEPS.tools_shared", pin_patch_text)
        self.assertIn(f'+        "commit": "{PUBLIC_TOOLS_SHARED_SHA}",', pin_patch_text)

    def test_wasm_jni_separates_short_backend_name_from_full_marker(self):
        self.assertIn('LYNX_ELEMENT_BRIDGE_WAMR_BACKEND_NAME="wasm"', ANDROID_CMAKE)
        self.assertIn(
            'LYNX_ELEMENT_BRIDGE_WAMR_BACKEND_MARKER="lynx-element-bridge-backend:wasm"',
            ANDROID_CMAKE,
        )
        self.assertIn("LYNX_ELEMENT_BRIDGE_WAMR_BACKEND_MARKER", ANDROID_JNI)
        self.assertIn("LYNX_ELEMENT_BRIDGE_WAMR_BACKEND_NAME", ANDROID_JNI)
        self.assertIn(
            "return env->NewStringUTF(LYNX_ELEMENT_BRIDGE_WAMR_BACKEND_NAME);",
            ANDROID_JNI,
        )

    def test_verify_creates_dependency_directory_before_temporary_checkout(self):
        create_parent = 'mkdir -p -- "$ROOT_DIR/.deps"'
        create_temp = 'mktemp -d "$ROOT_DIR/.deps/.tools-shared-verify.XXXXXX"'

        self.assertIn(create_parent, VERIFY_SCRIPT)
        self.assertLess(VERIFY_SCRIPT.index(create_parent), VERIFY_SCRIPT.index(create_temp))

    def test_all_native_apk_libraries_are_required_and_all_elfs_are_inspected(self):
        for library in (
            "liblynx_element_bridge_native.so",
            "liblynx_element_bridge_wamr.so",
            "liblynx_native_renderer.so",
            "liblynx_service_api.so",
            "liblynxbase.so",
            "liblynxgfx.so",
            "liblynxtrace.so",
        ):
            self.assertIn(f"  {library}\n", BUILD_SCRIPT)
        self.assertIn(
            'for elf_entry in "${native_apk_elf_entries[@]}"; do', BUILD_SCRIPT
        )
        self.assertIn('"$llvm_bin/llvm-readelf" -d "$native_elf"', BUILD_SCRIPT)
        self.assertIn(
            '"$llvm_bin/llvm-nm" -D -C --undefined-only "$native_elf"',
            BUILD_SCRIPT,
        )
        self.assertRegex(BUILD_SCRIPT, r"QuickJS.*Napi.*WebAssembly.*Wasm.*v8::.*V8")
        self.assertIn('[[ "$library" == liblynx_native_renderer.so ]]', BUILD_SCRIPT)
        self.assertIn("lynx_native_renderer_get_api$", BUILD_SCRIPT)

    def test_cleanup_is_best_effort_for_both_patch_series(self):
        cleanup = re.search(
            r"restore_lynx_source\(\) \{(?P<body>.*?)\n\}",
            BUILD_SCRIPT,
            re.DOTALL,
        )
        self.assertIsNotNone(cleanup)
        body = cleanup.group("body")
        self.assertIn("LYNX_TOOLS_SHARED_APPLIED_PATCH_FILES", body)
        self.assertIn("LYNX_APPLIED_PATCH_FILES", body)
        self.assertGreaterEqual(body.count("cleanup_status=1"), 2)
        self.assertNotIn("return 1", body)
        self.assertIn("trap - EXIT", body)
        self.assertIn('exit "$exit_status"', body)
        self.assertIn('exit "$cleanup_status"', body)

        for function_name in (
            "verify_lynx_tools_shared_patches",
            "verify_lynx_patches",
        ):
            function = re.search(
                rf"^{function_name}\(\) \{{(?P<body>.*?)^\}}$",
                VERIFY_SCRIPT,
                re.DOTALL | re.MULTILINE,
            )
            self.assertIsNotNone(function)
            reverse_loop = re.search(
                r"for \(\(i = \$\{#applied_patch_files\[@\]\} - 1;.*?done",
                function.group("body"),
                re.DOTALL,
            )
            self.assertIsNotNone(reverse_loop)
            self.assertIn("if ! git", reverse_loop.group(0))
            self.assertIn("apply_status=1", reverse_loop.group(0))
            self.assertNotIn("return", reverse_loop.group(0))

    def test_exit_trap_preserves_failure_and_reports_cleanup_failure(self):
        cleanup = re.search(
            r"restore_lynx_source\(\) \{.*?^\}$",
            BUILD_SCRIPT,
            re.DOTALL | re.MULTILINE,
        )
        self.assertIsNotNone(cleanup)
        script = f"""
set -euo pipefail
{cleanup.group(0)}
elf_inspect_dir=""
lynx_tools_shared_patches_applied=1
lynx_patches_applied=1
LYNX_TOOLS_SHARED_DIR=/tools-shared
LYNX_SOURCE_DIR=/lynx
LYNX_TOOLS_SHARED_APPLIED_PATCH_FILES=(tools-first.patch tools-fail.patch)
LYNX_APPLIED_PATCH_FILES=(lynx-first.patch lynx-fail.patch)
git() {{
  printf '%s\n' "${{@: -1}}"
  [[ "${{@: -1}}" != *-fail.patch ]]
}}
trap restore_lynx_source EXIT
exit "$1"
"""

        failed_command = subprocess.run(
            ["bash", "-c", script, "bash", "7"],
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(failed_command.returncode, 7)
        self.assertCountEqual(
            failed_command.stdout.splitlines(),
            (
                "tools-first.patch",
                "tools-fail.patch",
                "lynx-first.patch",
                "lynx-fail.patch",
            ),
        )

        cleanup_failure = subprocess.run(
            ["bash", "-c", script, "bash", "0"],
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(cleanup_failure.returncode, 1)

    def test_tools_shared_rejects_only_non_ignored_checkout_changes(self):
        self.assertIn(
            'status --porcelain=v1 --untracked-files=all', BUILD_SCRIPT
        )
        self.assertIn(
            'status --porcelain=v1 --untracked-files=all', VERIFY_SCRIPT
        )
        self.assertNotIn(
            'LYNX_TOOLS_SHARED_DIR" status --porcelain=v1 --untracked-files=no',
            BUILD_SCRIPT,
        )


if __name__ == "__main__":
    unittest.main()
