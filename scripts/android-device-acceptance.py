#!/usr/bin/env python3

import argparse
import hashlib
import json
import re
import struct
import subprocess
import time
import zipfile
import zlib
from pathlib import Path


PACKAGE = "com.yew.lynx.example"
COMPONENT = f"{PACKAGE}/.MainActivity"
TAG = "LynxElementBridge"
NATIVE_DIAGNOSTICS = (
    "Native renderer diagnostics mode=native "
    "bts_runtime=false mts_context=false template=false"
)
WASM_REPLACE_EXTRA = "com.yew.lynx.example.extra.REPLACE_WASM_MODULE"
WASM_REPLACEMENT_COUNT = 100
WASM_REPLACEMENT_ASSETS = {
    "wasm-dioxus": "dioxus_counter_replacement.wasm",
    "wasm-yew": "yew_counter_replacement.wasm",
}
REQUIRED_NATIVE_LIBRARIES = (
    "liblynx_native_renderer.so",
    "liblynx_element_bridge.so",
    "liblynxbase.so",
    "liblynxgfx.so",
    "liblynxtrace.so",
)


def adb(serial: str, *arguments: str, binary: bool = False):
    result = subprocess.run(
        ["adb", "-s", serial, *arguments],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return result.stdout if binary else result.stdout.decode("utf-8", errors="replace")


def shell(serial: str, *arguments: str) -> str:
    return adb(serial, "shell", *arguments).strip()


def launch(serial: str) -> None:
    shell(serial, "am", "force-stop", PACKAGE)
    output = shell(serial, "am", "start", "-W", "-n", COMPONENT)
    if "Status: ok" not in output:
        raise RuntimeError(f"Activity launch failed:\n{output}")


def decode_png(data: bytes):
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise RuntimeError("Android screencap returned an invalid PNG")
    position = 8
    width = 0
    height = 0
    compressed = []
    while position < len(data):
        if position + 12 > len(data):
            raise RuntimeError("Android screencap returned a truncated PNG chunk")
        length = struct.unpack_from(">I", data, position)[0]
        chunk_type = data[position + 4 : position + 8]
        chunk = data[position + 8 : position + 8 + length]
        position += length + 12
        if chunk_type == b"IHDR":
            width, height, depth, color, compression, filtering, interlace = struct.unpack(
                ">IIBBBBB", chunk
            )
            if (depth, color, compression, filtering, interlace) != (8, 6, 0, 0, 0):
                raise RuntimeError("Android screencap PNG is not non-interlaced RGBA8")
        elif chunk_type == b"IDAT":
            compressed.append(chunk)
        elif chunk_type == b"IEND":
            break
    if width == 0 or height == 0 or not compressed:
        raise RuntimeError("Android screencap PNG is missing image data")

    filtered = zlib.decompress(b"".join(compressed))
    stride = width * 4
    if len(filtered) != height * (stride + 1):
        raise RuntimeError("Android screencap PNG has an unexpected data length")
    pixels = bytearray(width * height * 4)
    previous = bytearray(stride)
    source = 0
    for y in range(height):
        filter_type = filtered[source]
        source += 1
        row = bytearray(filtered[source : source + stride])
        source += stride
        for x in range(stride):
            left = row[x - 4] if x >= 4 else 0
            above = previous[x]
            upper_left = previous[x - 4] if x >= 4 else 0
            if filter_type == 1:
                row[x] = (row[x] + left) & 0xFF
            elif filter_type == 2:
                row[x] = (row[x] + above) & 0xFF
            elif filter_type == 3:
                row[x] = (row[x] + (left + above) // 2) & 0xFF
            elif filter_type == 4:
                estimate = left + above - upper_left
                left_distance = abs(estimate - left)
                above_distance = abs(estimate - above)
                upper_left_distance = abs(estimate - upper_left)
                predictor = (
                    left
                    if left_distance <= above_distance and left_distance <= upper_left_distance
                    else above
                    if above_distance <= upper_left_distance
                    else upper_left
                )
                row[x] = (row[x] + predictor) & 0xFF
            elif filter_type != 0:
                raise RuntimeError(f"Unsupported PNG row filter {filter_type}")
        pixels[y * stride : (y + 1) * stride] = row
        previous = row
    return width, height, bytes(pixels)


def capture_screen(serial: str):
    png = adb(serial, "exec-out", "screencap", "-p", binary=True)
    return decode_png(png), png


def is_increment_color(pixels: bytes, offset: int) -> bool:
    red, green, blue, alpha = pixels[offset : offset + 4]
    return (
        alpha > 200
        and red <= 90
        and 75 <= green <= 145
        and 55 <= blue <= 125
        and green - red >= 15
        and green - blue >= 10
    )


def find_increment_button(screen):
    width, height, pixels = screen
    step = max(1, min(width, height) // 400)
    best_count = 0
    best_y = 0
    best_left = 0
    best_right = 0
    for y in range(0, height, step):
        count = 0
        left = width
        right = 0
        row = y * width * 4
        for x in range(0, width, step):
            if is_increment_color(pixels, row + x * 4):
                count += 1
                left = min(left, x)
                right = max(right, x)
        if count > best_count:
            best_count = count
            best_y = y
            best_left = left
            best_right = right

    if best_count * step < width // 2:
        raise RuntimeError("Increment control was not visible in the screenshot")

    def find_edge(probe_x: int, direction: int) -> int:
        y = best_y
        last_match = y
        misses = 0
        while 0 <= y < height and misses < 3:
            if is_increment_color(pixels, (y * width + probe_x) * 4):
                last_match = y
                misses = 0
            else:
                misses += 1
            y += direction
        return last_match

    span = best_right - best_left
    top, bottom = max(
        [
            (
                find_edge(best_left + span * numerator // 8, -1),
                find_edge(best_left + span * numerator // 8, 1),
            )
            for numerator in (1, 2, 4, 6, 7)
        ],
        key=lambda edges: edges[1] - edges[0],
    )
    if best_right - best_left < width // 2 or bottom - top < height // 12:
        raise RuntimeError("Detected green region does not match the Increment control")
    return best_left, top, best_right, bottom


def wait_for_page(serial: str, landscape: bool | None = None, timeout: float = 30.0):
    deadline = time.monotonic() + timeout
    last_error = "no screenshot captured"
    while time.monotonic() < deadline:
        try:
            screen, png = capture_screen(serial)
            if landscape is not None and (screen[0] > screen[1]) != landscape:
                last_error = "screen rotation has not completed"
                time.sleep(0.25)
                continue
            bounds = find_increment_button(screen)
            return screen, bounds, png
        except (subprocess.CalledProcessError, RuntimeError) as error:
            last_error = str(error)
            time.sleep(0.5)
    raise RuntimeError(f"Timed out waiting for the counter page: {last_error}")


def changed_pixels(before, after, left, top, right, bottom) -> int:
    if before[:2] != after[:2]:
        return 0
    width, _, before_pixels = before
    _, _, after_pixels = after
    changed = 0
    for y in range(top, bottom, 2):
        for x in range(left, right, 2):
            offset = (y * width + x) * 4
            if any(
                abs(before_pixels[offset + channel] - after_pixels[offset + channel]) > 12
                for channel in range(3)
            ):
                changed += 1
    return changed


def changed_counter_pixels(before, after, button_bounds) -> int:
    left, top, right, bottom = button_bounds
    button_height = bottom - top
    return changed_pixels(
        before, after, left, max(0, top - 2 * button_height), right, top
    )


def tap_increment(serial: str, screen, bounds):
    left, top, right, bottom = bounds
    shell(serial, "input", "tap", str((left + right) // 2), str((top + bottom) // 2))
    deadline = time.monotonic() + 30.0
    required_change = max(100, (right - left) * (bottom - top) // 5000)
    while time.monotonic() < deadline:
        updated, updated_bounds, png = wait_for_page(serial, timeout=2.0)
        if changed_counter_pixels(screen, updated, bounds) >= required_change:
            return updated, updated_bounds, png
        time.sleep(0.25)
    raise RuntimeError("Counter region did not change after tapping Increment")


def replace_wasm_module(serial: str, initial_screen, count_one_screen, bounds):
    output = shell(
        serial,
        "am",
        "start",
        "-W",
        "--activity-single-top",
        "-n",
        COMPONENT,
        "--ez",
        WASM_REPLACE_EXTRA,
        "true",
    )
    if "Status: ok" not in output:
        raise RuntimeError(f"WASM replacement intent failed:\n{output}")

    left, top, right, bottom = bounds
    required_change = max(100, (right - left) * (bottom - top) // 5000)
    deadline = time.monotonic() + 30.0
    while time.monotonic() < deadline:
        replaced, replaced_bounds, png = wait_for_page(serial, timeout=2.0)
        changed_from_one = changed_counter_pixels(count_one_screen, replaced, bounds)
        changed_from_initial = changed_counter_pixels(initial_screen, replaced, bounds)
        if changed_from_one >= required_change and changed_from_initial >= required_change:
            return replaced, replaced_bounds, png
        time.sleep(0.25)
    raise RuntimeError(
        f"Replacement fixture did not visibly render Count: {WASM_REPLACEMENT_COUNT} "
        "distinct from both Count: 0 and Count: 1"
    )


def screenshot(serial: str, output: Path) -> None:
    output.write_bytes(adb(serial, "exec-out", "screencap", "-p", binary=True))


def set_rotation(serial: str, rotation: int) -> None:
    shell(serial, "settings", "put", "system", "accelerometer_rotation", "0")
    shell(serial, "settings", "put", "system", "user_rotation", str(rotation))


def forbidden_library_kind(name: str) -> str | None:
    if name == "liblynx.so":
        return "stock"
    if name == "libquick.so":
        return "quick"
    if re.fullmatch(r"libnapi.*\.so", name):
        return "napi"
    if name == "libwasm.so":
        return "wasm"
    if name == "liblynx_v8_bridge.so" or re.fullmatch(r"libv8.*\.so", name):
        return "v8"
    return None


def inspect_apk(apk: Path) -> dict[str, tuple[int, int]]:
    try:
        with zipfile.ZipFile(apk) as archive:
            entries: dict[str, list[zipfile.ZipInfo]] = {}
            forbidden_entries = []
            for info in archive.infolist():
                entries.setdefault(info.filename, []).append(info)
                normalized_name = info.filename.replace("\\", "/")
                soname = normalized_name.rsplit("/", 1)[-1]
                if normalized_name == "assets/lynx_core.js" or forbidden_library_kind(
                    soname
                ):
                    forbidden_entries.append(info.filename)

            required_entries = {
                library: entries.get(f"lib/arm64-v8a/{library}", [])
                for library in REQUIRED_NATIVE_LIBRARIES
            }
            missing = [
                library for library, matches in required_entries.items() if not matches
            ]
            duplicates = [
                library for library, matches in required_entries.items() if len(matches) > 1
            ]
            if missing:
                raise RuntimeError(
                    "APK is missing required arm64 native libraries: "
                    + ", ".join(missing)
                )
            if duplicates:
                raise RuntimeError(
                    "APK contains duplicate required arm64 native libraries: "
                    + ", ".join(duplicates)
                )
            if forbidden_entries:
                raise RuntimeError(
                    "APK contains forbidden runtime artifacts: "
                    + ", ".join(sorted(forbidden_entries))
                )

            ranges = {}
            with apk.open("rb") as apk_file:
                for library, matches in required_entries.items():
                    info = matches[0]
                    if (
                        info.compress_type != zipfile.ZIP_STORED
                        or info.compress_size != info.file_size
                    ):
                        raise RuntimeError(
                            f"APK native library must be stored uncompressed: {library}"
                        )
                    if info.file_size == 0:
                        raise RuntimeError(f"APK native library is empty: {library}")

                    apk_file.seek(info.header_offset)
                    header = apk_file.read(30)
                    if len(header) != 30:
                        raise RuntimeError(f"APK local header is truncated: {library}")
                    (
                        signature,
                        _,
                        flags,
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                        name_length,
                        extra_length,
                    ) = struct.unpack("<IHHHHHIIIHH", header)
                    if signature != 0x04034B50:
                        raise RuntimeError(f"APK local header is invalid: {library}")
                    local_name = apk_file.read(name_length)
                    encoding = "utf-8" if flags & 0x800 else "cp437"
                    if local_name != info.orig_filename.encode(encoding):
                        raise RuntimeError(
                            f"APK local and central entry names differ: {library}"
                        )
                    data_offset = info.header_offset + 30 + name_length + extra_length
                    if data_offset % 4096 != 0:
                        raise RuntimeError(
                            f"APK native library is not page-aligned: {library}"
                        )
                    ranges[library] = (data_offset, data_offset + info.file_size)
            return ranges
    except (OSError, zipfile.BadZipFile) as error:
        raise RuntimeError(f"Unable to inspect APK ZIP: {error}") from error


def capture_process_maps(serial: str) -> str:
    pid_output = shell(serial, "pidof", PACKAGE)
    pids = pid_output.split()
    if len(pids) != 1 or not pids[0].isdigit():
        raise RuntimeError(f"Unable to identify one app process for {PACKAGE}")
    maps = shell(serial, "run-as", PACKAGE, "cat", f"/proc/{pids[0]}/maps")
    if not maps:
        raise RuntimeError(f"Process maps are unavailable for {PACKAGE}")
    return maps


def analyze_process_maps(
    maps: str, apk_library_ranges: dict[str, tuple[int, int]]
) -> tuple[list[str], list[str]]:
    mapped = set()
    forbidden = set()
    parsed_lines = 0
    for line in maps.splitlines():
        match = re.match(
            r"^[0-9a-fA-F]+-[0-9a-fA-F]+\s+\S+\s+([0-9a-fA-F]+)"
            r"\s+\S+\s+\d+(?:\s+(.*))?$",
            line,
        )
        if match is None:
            continue
        parsed_lines += 1
        file_offset = int(match.group(1), 16)
        pathname = (match.group(2) or "").strip()

        direct_sonames = re.findall(
            r"(?:^|[/!])([^/!\s]+\.so)(?=$|\s+\(deleted\)$)", pathname
        )
        for soname in direct_sonames:
            if soname in apk_library_ranges:
                mapped.add(soname)
            if forbidden_library_kind(soname):
                forbidden.add(soname)

        if re.search(r"(?:^|/)base\.apk(?:\s+\(deleted\))?$", pathname):
            for library, (start, end) in apk_library_ranges.items():
                if start <= file_offset < end:
                    mapped.add(library)

    if parsed_lines == 0:
        raise RuntimeError("Process maps did not contain any parseable mappings")
    return (
        [library for library in REQUIRED_NATIVE_LIBRARIES if library in mapped],
        sorted(forbidden),
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Run Lynx Element Bridge device acceptance")
    parser.add_argument(
        "--backend",
        choices=("yew", "dioxus", "wasm-dioxus", "wasm-yew"),
        default="yew",
    )
    parser.add_argument("--serial", required=True, help="ADB serial; never written to evidence")
    parser.add_argument("--apk", type=Path, required=True)
    parser.add_argument("--evidence-dir", type=Path, required=True)
    parser.add_argument("--cycles", type=int, default=3)
    args = parser.parse_args()

    if args.cycles < 1:
        parser.error("--cycles must be positive")
    apk = args.apk.resolve()
    if not apk.is_file():
        parser.error(f"APK does not exist: {apk}")
    evidence = args.evidence_dir.resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    (evidence / "after-timer-fired.png").unlink(missing_ok=True)
    for name in (
        "fresh-count-0.png",
        "after-tap-count-1.png",
        "after-wasm-replace-count-100.png",
        "after-activity-recreation.png",
        "after-force-stop-reopen.png",
        "logcat.txt",
        "maps-fresh.txt",
        "maps-after-interaction.txt",
        "summary.json",
    ):
        (evidence / name).unlink(missing_ok=True)

    apk_library_ranges = inspect_apk(apk)
    serial = args.serial
    adb(serial, "get-state")
    abi = shell(serial, "getprop", "ro.product.cpu.abi")
    api = shell(serial, "getprop", "ro.build.version.sdk")
    if abi != "arm64-v8a":
        raise RuntimeError(f"Acceptance requires arm64-v8a, device reports {abi!r}")

    original_accelerometer = shell(serial, "settings", "get", "system", "accelerometer_rotation")
    original_rotation = shell(serial, "settings", "get", "system", "user_rotation")
    initial_rotation = int(original_rotation) if original_rotation.isdigit() else 0
    alternate_rotation = 1 if initial_rotation == 0 else 0

    adb(serial, "logcat", "-c")
    adb(serial, "install", "-r", str(apk))
    try:
        launch(serial)
        screen, button, png = wait_for_page(serial)
        initial_screen = screen
        (evidence / "fresh-count-0.png").write_bytes(png)
        fresh_maps = capture_process_maps(serial)
        (evidence / "maps-fresh.txt").write_text(fresh_maps + "\n", encoding="utf-8")
        fresh_mapped, fresh_forbidden = analyze_process_maps(
            fresh_maps, apk_library_ranges
        )
        fresh_missing = [
            library
            for library in REQUIRED_NATIVE_LIBRARIES
            if library not in fresh_mapped
        ]
        if fresh_missing or fresh_forbidden:
            raise RuntimeError(
                "Fresh process maps failed native library checks: "
                f"missing={fresh_missing}, forbidden={fresh_forbidden}"
            )

        count_one_screen, button, png = tap_increment(serial, screen, button)
        (evidence / "after-tap-count-1.png").write_bytes(png)
        wasm_replacement_detected = False
        if args.backend.startswith("wasm-"):
            screen, button, png = replace_wasm_module(
                serial, initial_screen, count_one_screen, button
            )
            (evidence / "after-wasm-replace-count-100.png").write_bytes(png)
            wasm_replacement_detected = True
        after_interaction_maps = capture_process_maps(serial)
        (evidence / "maps-after-interaction.txt").write_text(
            after_interaction_maps + "\n", encoding="utf-8"
        )
        after_interaction_mapped, after_interaction_forbidden = (
            analyze_process_maps(after_interaction_maps, apk_library_ranges)
        )
        after_interaction_missing = [
            library
            for library in REQUIRED_NATIVE_LIBRARIES
            if library not in after_interaction_mapped
        ]
        if after_interaction_missing or after_interaction_forbidden:
            raise RuntimeError(
                "Post-interaction process maps failed native library checks: "
                f"missing={after_interaction_missing}, "
                f"forbidden={after_interaction_forbidden}"
            )

        set_rotation(serial, alternate_rotation)
        wait_for_page(serial, landscape=alternate_rotation in {1, 3})
        screenshot(serial, evidence / "after-activity-recreation.png")

        launch(serial)
        # OEMs may reset the display rotation while force-stopping the process.
        wait_for_page(serial)
        screenshot(serial, evidence / "after-force-stop-reopen.png")

        for cycle in range(1, args.cycles + 1):
            launch(serial)
            screen, button, _ = wait_for_page(serial)
            tap_increment(serial, screen, button)
            rotation = 0 if screen[0] > screen[1] else 1
            set_rotation(serial, rotation)
            wait_for_page(serial, landscape=rotation in {1, 3})

        logs = adb(serial, "logcat", "-d", "-v", "threadtime")
        log_lines = logs.splitlines()
        tag_lines = [line for line in log_lines if TAG in line]
        on_create_count = sum("MainActivity onCreate" in line for line in tag_lines)
        on_destroy_count = sum("MainActivity onDestroy complete" in line for line in tag_lines)
        if on_create_count < args.cycles + 2 or on_destroy_count < args.cycles:
            raise RuntimeError(
                f"Lifecycle evidence incomplete: onCreate={on_create_count}, "
                f"onDestroy={on_destroy_count}"
            )
        expected_diagnostics = NATIVE_DIAGNOSTICS.replace(
            "mode=native",
            f"mode={'wasm' if args.backend.startswith('wasm-') else 'native'}",
        )
        diagnostics_count = sum(expected_diagnostics in line for line in tag_lines)
        if diagnostics_count != on_create_count:
            raise RuntimeError(
                "Runtime diagnostics incomplete: "
                f"diagnostics={diagnostics_count}, onCreate={on_create_count}"
            )
        app_pids = {
            match.group(1)
            for line in tag_lines
            if (match := re.match(r"^\S+\s+\S+\s+(\d+)\s+\d+\s", line))
        }
        relevant_lines = []
        for line in log_lines:
            match = re.match(r"^\S+\s+\S+\s+(\d+)\s+\d+\s", line)
            if TAG in line or PACKAGE in line or (match is not None and match.group(1) in app_pids):
                relevant_lines.append(line)
        relevant_logs = "\n".join(relevant_lines)
        (evidence / "logcat.txt").write_text(relevant_logs + "\n", encoding="utf-8")
        crash_markers = ("FATAL EXCEPTION", "Fatal signal", "native_bridge_failure")
        if any(marker in relevant_logs for marker in crash_markers):
            raise RuntimeError("Crash or native bridge failure found in logcat evidence")
        backend_marker = f"Native renderer backend={args.backend.removeprefix('wasm-')}"
        if backend_marker not in relevant_logs:
            raise RuntimeError(f"Expected backend identity was not logged: {backend_marker}")
        if args.backend.startswith("wasm-"):
            replacement_log = (
                "WASM module replacement complete "
                f"asset={WASM_REPLACEMENT_ASSETS[args.backend]}"
            )
            if replacement_log not in relevant_logs:
                raise RuntimeError(
                    "Expected WASM replacement asset completion was not logged: "
                    f"{replacement_log}"
                )

        mapped_forbidden_libraries = sorted(
            set(fresh_forbidden) | set(after_interaction_forbidden)
        )
        summary = {
            "backend": args.backend,
            "renderer_mode": (
                "wasm" if args.backend.startswith("wasm-") else "native"
            ),
            "bts_runtime": False,
            "mts_context": False,
            "template": False,
            "native_renderer_diagnostics_count": diagnostics_count,
            "apk_sha256": hashlib.sha256(apk.read_bytes()).hexdigest(),
            "device_abi": abi,
            "device_api": api,
            "cycles": args.cycles,
            "activity_on_create_count": on_create_count,
            "activity_on_destroy_count": on_destroy_count,
            "fresh_page_detected": True,
            "tap_visual_change_detected": True,
            "wasm_replacement_expected_count": (
                WASM_REPLACEMENT_COUNT if args.backend.startswith("wasm-") else None
            ),
            "wasm_replacement_asset": WASM_REPLACEMENT_ASSETS.get(args.backend),
            "wasm_replacement_candidate_detected": wasm_replacement_detected,
            "activity_recreation_page_detected": True,
            "force_stop_reopen_page_detected": True,
            "proc_maps_checked": True,
            "fresh_required_libraries": list(REQUIRED_NATIVE_LIBRARIES),
            "fresh_mapped_libraries": fresh_mapped,
            "after_interaction_required_libraries": list(
                REQUIRED_NATIVE_LIBRARIES
            ),
            "after_interaction_mapped_libraries": after_interaction_mapped,
            "mapped_forbidden_libraries": mapped_forbidden_libraries,
            "quick_mapped": any(
                forbidden_library_kind(library) == "quick"
                for library in mapped_forbidden_libraries
            ),
            "napi_mapped": any(
                forbidden_library_kind(library) == "napi"
                for library in mapped_forbidden_libraries
            ),
            "wasm_mapped": any(
                forbidden_library_kind(library) == "wasm"
                for library in mapped_forbidden_libraries
            ),
            "v8_mapped": any(
                forbidden_library_kind(library) == "v8"
                for library in mapped_forbidden_libraries
            ),
            "text_evidence_requires_visual_review": True,
        }
        (evidence / "summary.json").write_text(
            json.dumps(summary, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        print(json.dumps(summary, indent=2, sort_keys=True))
    finally:
        shell(serial, "am", "force-stop", PACKAGE)
        if original_accelerometer not in {"", "null"}:
            shell(serial, "settings", "put", "system", "accelerometer_rotation", original_accelerometer)
        if original_rotation not in {"", "null"}:
            shell(serial, "settings", "put", "system", "user_rotation", original_rotation)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
