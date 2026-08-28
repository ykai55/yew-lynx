#!/usr/bin/env python3

import importlib.util
import struct
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).with_name("android-device-acceptance.py")
SPEC = importlib.util.spec_from_file_location("android_device_acceptance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
acceptance = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(acceptance)


class AndroidDeviceAcceptanceTest(unittest.TestCase):
    def setUp(self):
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary_directory.cleanup)
        self.directory = Path(self.temporary_directory.name)

    def add_entry(
        self,
        archive,
        name,
        data=b"data",
        compression=zipfile.ZIP_STORED,
        aligned=True,
    ):
        info = zipfile.ZipInfo(name)
        info.compress_type = compression
        if aligned:
            header_offset = archive.fp.tell()
            padding = -(header_offset + 30 + len(name.encode("ascii")) + 4) % 4096
            info.extra = struct.pack("<HH", 0xCAFE, padding) + bytes(padding)
        archive.writestr(info, data)

    def write_apk(self, name="test.apk", *, extra_entries=(), compression=None):
        path = self.directory / name
        with zipfile.ZipFile(path, "w") as archive:
            for library in acceptance.REQUIRED_NATIVE_LIBRARIES:
                data = (b"\x7fELF" + library.encode("ascii")).ljust(0x3000, b"x")
                self.add_entry(
                    archive,
                    f"lib/arm64-v8a/{library}",
                    data,
                    (compression or {}).get(library, zipfile.ZIP_STORED),
                )
            for entry in extra_entries:
                self.add_entry(archive, entry)
        return path

    def maps_for_ranges(self, ranges, overrides=None):
        overrides = overrides or {}
        lines = []
        for index, (library, (start, _)) in enumerate(ranges.items()):
            offset = overrides.get(library, start + 0x1000)
            lines.append(
                f"7a{index:08x}-7a{index + 1:08x} r-xp {offset:08x} "
                "fd:01 123 /data/app/example/base.apk"
            )
        return "\n".join(lines)

    def test_apk_offsets_correlate_base_apk_maps(self):
        ranges = acceptance.inspect_apk(self.write_apk())

        self.assertEqual(
            list(ranges), list(acceptance.REQUIRED_NATIVE_LIBRARIES)
        )
        for start, end in ranges.values():
            self.assertEqual(start % 4096, 0)
            self.assertEqual(end - start, 0x3000)

        mapped, forbidden = acceptance.analyze_process_maps(
            self.maps_for_ranges(ranges), ranges
        )
        self.assertEqual(mapped, list(acceptance.REQUIRED_NATIVE_LIBRARIES))
        self.assertEqual(forbidden, [])

    def test_map_offset_end_boundary_does_not_match_zip_entry(self):
        ranges = acceptance.inspect_apk(self.write_apk())
        renderer = acceptance.REQUIRED_NATIVE_LIBRARIES[0]
        maps = self.maps_for_ranges(ranges, {renderer: ranges[renderer][1]})

        mapped, _ = acceptance.analyze_process_maps(maps, ranges)

        self.assertNotIn(renderer, mapped)
        self.assertEqual(
            mapped, list(acceptance.REQUIRED_NATIVE_LIBRARIES[1:])
        )

    def test_direct_sonames_detect_required_and_forbidden_libraries(self):
        ranges = acceptance.inspect_apk(self.write_apk())
        paths = [
            "/data/app/liblynx_native_renderer.so",
            "/data/app/liblynx_element_bridge.so (deleted)",
            "/data/app/liblynxbase.so",
            "/data/app/liblynxgfx.so",
            "/data/app/liblynxtrace.so",
            "/data/app/liblynx.so",
            "/data/app/libquick.so",
            "/data/app/libnapi_runtime.so",
            "/data/app/libwasm.so",
            "/data/app/liblynx_v8_bridge.so",
            "/data/app/libv8_libfull.so",
        ]
        maps = "\n".join(
            f"7b{index:08x}-7b{index + 1:08x} r-xp 00000000 fd:01 123 {path}"
            for index, path in enumerate(paths)
        )

        mapped, forbidden = acceptance.analyze_process_maps(maps, ranges)

        self.assertEqual(mapped, list(acceptance.REQUIRED_NATIVE_LIBRARIES))
        self.assertEqual(
            forbidden,
            [
                "liblynx.so",
                "liblynx_v8_bridge.so",
                "libnapi_runtime.so",
                "libquick.so",
                "libv8_libfull.so",
                "libwasm.so",
            ],
        )

    def test_apk_rejects_missing_compressed_unaligned_and_forbidden_entries(self):
        missing = self.directory / "missing.apk"
        with zipfile.ZipFile(missing, "w") as archive:
            for library in acceptance.REQUIRED_NATIVE_LIBRARIES[:-1]:
                self.add_entry(archive, f"lib/arm64-v8a/{library}")

        compressed_library = acceptance.REQUIRED_NATIVE_LIBRARIES[0]
        compressed = self.write_apk(
            "compressed.apk", compression={compressed_library: zipfile.ZIP_DEFLATED}
        )

        unaligned = self.directory / "unaligned.apk"
        with zipfile.ZipFile(unaligned, "w") as archive:
            for library in acceptance.REQUIRED_NATIVE_LIBRARIES:
                self.add_entry(
                    archive, f"lib/arm64-v8a/{library}", aligned=False
                )

        forbidden = self.write_apk(
            "forbidden.apk",
            extra_entries=(
                "lib/arm64-v8a/libnapi_runtime.so",
                "assets/lynx_core.js",
            ),
        )

        for apk in (missing, compressed, unaligned, forbidden):
            with self.subTest(apk=apk.name), self.assertRaises(RuntimeError):
                acceptance.inspect_apk(apk)

    def test_process_maps_capture_uses_run_as_and_fails_closed(self):
        with mock.patch.object(
            acceptance, "shell", side_effect=["1234", "one mapping"]
        ) as shell:
            self.assertEqual(acceptance.capture_process_maps("serial"), "one mapping")
        self.assertEqual(
            shell.call_args_list,
            [
                mock.call("serial", "pidof", acceptance.PACKAGE),
                mock.call(
                    "serial",
                    "run-as",
                    acceptance.PACKAGE,
                    "cat",
                    "/proc/1234/maps",
                ),
            ],
        )

        for outputs in ([""], ["1234 5678"], ["1234", ""]):
            with self.subTest(outputs=outputs), mock.patch.object(
                acceptance, "shell", side_effect=outputs
            ), self.assertRaises(RuntimeError):
                acceptance.capture_process_maps("serial")

        with self.assertRaises(RuntimeError):
            acceptance.analyze_process_maps("not a maps line", {})

    def test_increment_button_uses_intact_edge_when_text_cuts_center_probes(self):
        width = height = 400
        pixels = bytearray(bytes([255, 255, 255, 255]) * width * height)
        green = bytes([50, 110, 80, 255])
        for y in range(100, 301):
            for x in range(50, 351):
                pixels[(y * width + x) * 4 : (y * width + x + 1) * 4] = green
        for y in range(129, 272):
            for x in range(120, 281):
                pixels[(y * width + x) * 4 : (y * width + x + 1) * 4] = bytes(
                    [255, 255, 255, 255]
                )

        self.assertEqual(
            acceptance.find_increment_button((width, height, bytes(pixels))),
            (50, 100, 350, 300),
        )

    def test_wasm_replacement_requires_candidate_distinct_from_both_prior_states(self):
        initial = (100, 100, bytes([0, 0, 0, 255]) * 10_000)
        count_one_pixels = bytearray(initial[2])
        for y in range(20, 40, 2):
            for x in range(10, 90, 2):
                count_one_pixels[(y * 100 + x) * 4] = 255
        count_one = (100, 100, bytes(count_one_pixels))
        replacement_pixels = bytearray(initial[2])
        for y in range(20, 40, 2):
            for x in range(10, 90, 2):
                replacement_pixels[(y * 100 + x) * 4 + 1] = 255
        replacement = (100, 100, bytes(replacement_pixels))
        png = b"replacement screenshot"

        with mock.patch.object(
            acceptance, "shell", return_value="Status: ok"
        ) as shell, mock.patch.object(
            acceptance,
            "wait_for_page",
            return_value=(replacement, (10, 40, 90, 80), png),
        ):
            replaced = acceptance.replace_wasm_module(
                "serial", initial, count_one, (10, 40, 90, 80)
            )

        self.assertEqual(replaced, (replacement, (10, 40, 90, 80), png))
        shell.assert_called_once_with(
            "serial",
            "am",
            "start",
            "-W",
            "--activity-single-top",
            "-n",
            acceptance.COMPONENT,
            "--ez",
            acceptance.WASM_REPLACE_EXTRA,
            "true",
        )

    def test_wasm_replacement_rejects_the_initial_fixture(self):
        initial = (100, 100, bytes([0, 0, 0, 255]) * 10_000)
        count_one_pixels = bytearray(initial[2])
        for y in range(20, 40, 2):
            for x in range(10, 90, 2):
                count_one_pixels[(y * 100 + x) * 4] = 255

        with mock.patch.object(
            acceptance, "shell", return_value="Status: ok"
        ), mock.patch.object(
            acceptance,
            "wait_for_page",
            return_value=(initial, (10, 40, 90, 80), b"initial screenshot"),
        ), mock.patch.object(
            acceptance.time, "monotonic", side_effect=[0.0, 31.0]
        ), self.assertRaisesRegex(RuntimeError, "Count: 100"):
            acceptance.replace_wasm_module(
                "serial", initial, (100, 100, bytes(count_one_pixels)), (10, 40, 90, 80)
            )


if __name__ == "__main__":
    unittest.main()
