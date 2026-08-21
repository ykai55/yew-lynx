#!/usr/bin/env python3

import argparse
import hashlib
import json
import re
import struct
import subprocess
import time
import zlib
from pathlib import Path


PACKAGE = "com.yew.lynx.example"
COMPONENT = f"{PACKAGE}/.MainActivity"
TAG = "LynxElementBridge"
NATIVE_DIAGNOSTICS = (
    "Native renderer diagnostics mode=native "
    "bts_runtime=false mts_context=false template=false"
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

    probe_x = best_left + (best_right - best_left) // 4

    def find_edge(direction: int) -> int:
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

    top = find_edge(-1)
    bottom = find_edge(1)
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


def changed_timer_pixels(before, after, button_bounds) -> int:
    left, top, right, bottom = button_bounds
    button_height = bottom - top
    return changed_pixels(before, after, left, max(0, top - button_height), right, top)


def changed_counter_pixels(before, after, button_bounds) -> int:
    left, top, right, bottom = button_bounds
    button_height = bottom - top
    return changed_pixels(
        before, after, left, max(0, top - 2 * button_height), right, top
    )


def wait_for_timer_change(serial: str, screen, bounds, timeout: float = 10.0):
    left, top, right, bottom = bounds
    required_change = max(50, (right - left) * (bottom - top) // 5000)
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        updated, updated_bounds, png = wait_for_page(serial, timeout=2.0)
        if changed_timer_pixels(screen, updated, bounds) >= required_change:
            return updated, updated_bounds, png
        time.sleep(0.1)
    raise RuntimeError("Timer region did not change before tapping Increment")


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


def screenshot(serial: str, output: Path) -> None:
    output.write_bytes(adb(serial, "exec-out", "screencap", "-p", binary=True))


def set_rotation(serial: str, rotation: int) -> None:
    shell(serial, "settings", "put", "system", "accelerometer_rotation", "0")
    shell(serial, "settings", "put", "system", "user_rotation", str(rotation))


def main() -> int:
    parser = argparse.ArgumentParser(description="Run Lynx Element Bridge device acceptance")
    parser.add_argument("--backend", choices=("yew", "dioxus"), default="yew")
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
    for name in (
        "fresh-count-0.png",
        "after-timer-fired.png",
        "after-tap-count-1.png",
        "after-activity-recreation.png",
        "after-force-stop-reopen.png",
        "logcat.txt",
        "summary.json",
    ):
        (evidence / name).unlink(missing_ok=True)

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
        (evidence / "fresh-count-0.png").write_bytes(png)

        screen, button, png = wait_for_timer_change(serial, screen, button)
        (evidence / "after-timer-fired.png").write_bytes(png)

        _, _, png = tap_increment(serial, screen, button)
        (evidence / "after-tap-count-1.png").write_bytes(png)

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
        diagnostics_count = sum(NATIVE_DIAGNOSTICS in line for line in tag_lines)
        if diagnostics_count != on_create_count:
            raise RuntimeError(
                "Runtime-native diagnostics incomplete: "
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
        backend_marker = f"Native renderer backend={args.backend}"
        if backend_marker not in relevant_logs:
            raise RuntimeError(f"Expected backend identity was not logged: {backend_marker}")

        summary = {
            "backend": args.backend,
            "renderer_mode": "native",
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
            "timer_visual_change_detected": True,
            "tap_visual_change_detected": True,
            "activity_recreation_page_detected": True,
            "force_stop_reopen_page_detected": True,
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
