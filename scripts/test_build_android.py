#!/usr/bin/env python3

import pathlib
import re
import subprocess
import unittest


ROOT = pathlib.Path(__file__).resolve().parent.parent
BUILD_SCRIPT = (ROOT / "scripts" / "build-android.sh").read_text()
VERIFY_SCRIPT = (ROOT / "scripts" / "verify.sh").read_text()
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

    def test_wamr_mode_is_static_pinned_and_packages_generated_guest(self):
        self.assertIn(f'WAMR_SHA="{WAMR_SHA}"', BUILD_SCRIPT)
        self.assertIn(f'WAMR_SHA="{WAMR_SHA}"', VERIFY_SCRIPT)
        self.assertIn("wasm-dioxus", BUILD_SCRIPT)
        self.assertIn("wasm-yew", BUILD_SCRIPT)
        self.assertIn("dioxus_counter.wasm", BUILD_SCRIPT)
        self.assertIn("dioxus_counter_replacement.wasm", BUILD_SCRIPT)
        self.assertIn("yew_counter.wasm", BUILD_SCRIPT)
        self.assertIn("yew_counter_replacement.wasm", BUILD_SCRIPT)
        self.assertIn("libwasm\\.so", BUILD_SCRIPT)

    def test_wasm_fixtures_share_each_framework_crate_and_use_a_feature(self):
        gradle = (ROOT / "examples" / "android" / "app" / "build.gradle.kts").read_text()

        self.assertIn('"--features", "replacement-fixture"', gradle)
        self.assertIn('target/wasm-guests/$elementBridgeBackend', gradle)
        self.assertIn('resolve("initial")', gradle)
        self.assertIn('resolve("replacement")', gradle)
        self.assertNotIn("replacement-counter", gradle)

    def test_android_ci_covers_native_and_wasm_backends(self):
        workflow = (ROOT / ".github" / "workflows" / "android-integration.yml").read_text()

        self.assertIn("backend: [yew, dioxus, wasm-dioxus, wasm-yew]", workflow)
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
        self.assertIn('LYNX_ELEMENT_BRIDGE_WAMR_BACKEND_NAME="${wamr_backend_name}"', ANDROID_CMAKE)
        self.assertIn(
            'LYNX_ELEMENT_BRIDGE_WAMR_BACKEND_MARKER="lynx-element-bridge-backend:${LYNX_ELEMENT_BRIDGE_BACKEND}"',
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
            "liblynx_element_bridge.so",
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
