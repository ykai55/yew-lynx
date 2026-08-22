#!/usr/bin/env python3

import pathlib
import re
import subprocess
import unittest


ROOT = pathlib.Path(__file__).resolve().parent.parent
BUILD_SCRIPT = (ROOT / "scripts" / "build-android.sh").read_text()
VERIFY_SCRIPT = (ROOT / "scripts" / "verify.sh").read_text()


class BuildAndroidStaticTest(unittest.TestCase):
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
