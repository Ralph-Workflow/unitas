from __future__ import annotations

import re
import struct
import subprocess
import sys
import time
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_REFERENCE_BMP = REPO_ROOT / "tools" / "approved_gpu_reference.bmp"
DEFAULT_TOLERANCE = 4.0
TOLERANCE_EPSILON = 1e-6
DEFAULT_BUDGET_SECONDS = 120.0
BUILD_BUDGET_SECONDS = 600.0
BMP_HEADER_SIZE = 54
BMP_MIN_DIB_HEADER_SIZE = 40
BMP_BPP_24 = 24
DEVICE_FRAMES = 12
COMPARE_ARG_COUNT = 3
COMPARE_WITH_TOLERANCE_ARG_COUNT = 5
ACTUAL_ARG_COUNT = 2
ACTUAL_CAPTURE_PATH = REPO_ROOT / "target" / "device-lane" / "actual_gpu_frame.bmp"
PLAYER_BINARY = REPO_ROOT / "target" / "debug" / "unitas-player"
POLICY_CITATION = "docs/ralph-workflow-policy/verification-policy.md § Gate lanes"
TIMEOUT_CITATION = "docs/ralph-workflow-policy/verification-policy.md § Default requirements"
DEVICE_PACKAGES: tuple[str, ...] = (
    "unitas-render",
    "unitas-audio",
    "unitas-runtime",
    "unitas-build",
)

LANE_METADATA: tuple[tuple[str, str], ...] = (
    ("owner", "wt-03"),
    ("trigger", "every milestone touching presentation"),
    ("lane", "make verify-device"),
    ("evidence", "lane exit status and captured GPU/audio/input artifacts"),
    ("review", "when the player gains a new platform"),
)

DEVICE_REPORT_FIELDS: tuple[tuple[str, str], ...] = (
    ("size", r"size=(\d+x\d+)"),
    ("frames_presented", r"frames_presented=(\d+)"),
    ("mean_error", r"mean_error=([0-9.]+)"),
    ("tolerance", r"tolerance=([0-9.]+)"),
    ("audio_active", r"audio_active=(true|false)"),
    ("audio_frames", r"audio_frames=(\d+)"),
    ("ball_moved", r"ball_moved=(true|false)"),
)


def help_text() -> str:
    metadata_lines = [f"  {key}: {value}" for key, value in LANE_METADATA]
    usage = (
        "usage: verify_device.py [--help] [--metadata] [--actual <capture.bmp>]\n"
        "       verify_device.py --compare <actual.bmp> <reference.bmp> [--tolerance <float>]"
    )
    return "\n".join(
        [
            usage,
            "lane metadata:",
            *metadata_lines,
        ]
    )


def metadata_text() -> str:
    return "\n".join(f"{key}: {value}" for key, value in LANE_METADATA)


def parse_bmp_rgb(data: bytes) -> tuple[int, int, list[tuple[int, int, int]]]:
    if len(data) < BMP_HEADER_SIZE or data[:2] != b"BM":
        message = "invalid BMP header signature"
        raise ValueError(message)
    pixel_offset = struct.unpack_from("<I", data, 10)[0]
    dib_size = struct.unpack_from("<I", data, 14)[0]
    if dib_size < BMP_MIN_DIB_HEADER_SIZE:
        message = f"unsupported BMP DIB header size {dib_size}"
        raise ValueError(message)
    width, raw_height = struct.unpack_from("<ii", data, 18)
    planes, bpp = struct.unpack_from("<HH", data, 26)
    compression = struct.unpack_from("<I", data, 30)[0]
    if planes != 1 or bpp != BMP_BPP_24 or compression != 0:
        message = f"unsupported BMP format planes={planes} bpp={bpp} compression={compression}"
        raise ValueError(message)
    if width <= 0 or raw_height == 0:
        message = f"invalid BMP dimensions {width}x{raw_height}"
        raise ValueError(message)
    top_down = raw_height < 0
    height = abs(raw_height)
    row_stride = ((width * 3) + 3) & ~3
    required_size = pixel_offset + (row_stride * height)
    if len(data) < required_size:
        message = f"truncated BMP payload: {len(data)} < {required_size}"
        raise ValueError(message)
    pixels: list[tuple[int, int, int]] = [(0, 0, 0)] * (width * height)
    for row_index in range(height):
        src_row = row_index if top_down else (height - 1 - row_index)
        row_start = pixel_offset + (src_row * row_stride)
        for col_index in range(width):
            offset = row_start + (col_index * 3)
            blue = data[offset]
            green = data[offset + 1]
            red = data[offset + 2]
            pixels[(row_index * width) + col_index] = (red, green, blue)
    return (width, height, pixels)


def compare_bmp_files(
    actual_path: Path,
    reference_path: Path,
    tolerance: float = DEFAULT_TOLERANCE,
) -> tuple[bool, float, int, str]:
    if not actual_path.is_file():
        return (False, 0.0, 0, f"captured image missing at {actual_path}")
    if not reference_path.is_file():
        return (False, 0.0, 0, f"approved reference missing at {reference_path}")
    try:
        actual_w, actual_h, actual_pixels = parse_bmp_rgb(actual_path.read_bytes())

        ref_w, ref_h, ref_pixels = parse_bmp_rgb(reference_path.read_bytes())
    except (OSError, ValueError) as error:
        return (False, 0.0, 0, f"BMP parse error: {error}")
    if (actual_w, actual_h) != (ref_w, ref_h):
        return (
            False,
            0.0,
            0,
            f"dimension mismatch: actual {actual_w}x{actual_h} vs reference {ref_w}x{ref_h}",
        )
    total_diff = 0
    max_diff = 0
    channel_count = len(actual_pixels) * 3
    for actual_rgb, ref_rgb in zip(actual_pixels, ref_pixels, strict=True):
        for actual_c, ref_c in zip(actual_rgb, ref_rgb, strict=True):
            diff = abs(actual_c - ref_c)
            total_diff += diff
            max_diff = max(max_diff, diff)
    mean_error = (total_diff / channel_count) if channel_count > 0 else 0.0
    if mean_error > tolerance:
        summary = (
            f"mean channel error {mean_error:.4f} exceeds tolerance "
            f"{tolerance:.4f} (max_diff={max_diff})"
        )
        return (False, mean_error, max_diff, summary)
    summary = (
        f"mean channel error {mean_error:.4f} within tolerance "
        f"{tolerance:.4f} (max_diff={max_diff})"
    )
    return (True, mean_error, max_diff, summary)


def run_step(
    label: str,
    command: Sequence[str],
    budget_seconds: float,
) -> tuple[int, str]:
    started = time.monotonic()
    try:
        completed = subprocess.run(
            list(command),
            cwd=REPO_ROOT,
            check=False,
            timeout=budget_seconds,
            capture_output=True,
            text=True,
            errors="replace",
        )
    except subprocess.TimeoutExpired:
        elapsed = time.monotonic() - started
        sys.stderr.write(
            f"FAIL verify-device ({label}): timed out after {elapsed:.2f}s "
            f"(budget {budget_seconds:.0f}s); a slow check is a defect to diagnose, "
            f"never a budget to raise ({TIMEOUT_CITATION})\n"
        )
        return (1, "")
    elapsed = time.monotonic() - started
    output = completed.stdout + completed.stderr
    if output:
        sys.stdout.write(output if output.endswith("\n") else output + "\n")
    sys.stdout.write(f"device-step {label} took {elapsed:.2f}s (exit {completed.returncode})\n")
    if completed.returncode != 0:
        cmd_str = " ".join(command)
        sys.stderr.write(
            f"FAIL verify-device ({label}): command {cmd_str} exited "
            f"{completed.returncode}; every declared profile must pass hard "
            f"when run ({POLICY_CITATION})\n"
        )
    return (completed.returncode, output)


def parse_device_report(output: str) -> dict[str, str] | None:
    line = next(
        (line for line in output.splitlines() if line.startswith("device_verify=ok")),
        None,
    )
    if line is None:
        return None
    report: dict[str, str] = {}
    for name, pattern in DEVICE_REPORT_FIELDS:
        match = re.search(pattern, line)
        if match is None:
            return None
        report[name] = match.group(1)
    return report


def device_failure_reason(report: dict[str, str] | None) -> str | None:
    if report is None:
        return "the player reported no device verification line"
    frames = int(report["frames_presented"])
    tolerance = float(report["tolerance"])
    mean_error = float(report["mean_error"])
    audio_frames = int(report["audio_frames"])
    checks: tuple[tuple[bool, str], ...] = (
        (frames < 1, f"the window presented {frames} frames; at least 1 is required"),
        (
            abs(tolerance - DEFAULT_TOLERANCE) > TOLERANCE_EPSILON,
            f"the player compared at tolerance {tolerance:.4f}; "
            f"the lane declares {DEFAULT_TOLERANCE:.4f}",
        ),
        (
            mean_error > tolerance,
            f"GPU frame mean channel error {mean_error:.4f} exceeds tolerance {tolerance:.4f}",
        ),
        (
            report["audio_active"] != "true",
            "the default audio output device was not active",
        ),
        (
            audio_frames < 1,
            f"the tone pushed {audio_frames} frames to the audio device; at least 1 is required",
        ),
        (
            report["ball_moved"] != "true",
            "the synthetic input events did not move the sample scene ball",
        ),
    )
    for failed, reason in checks:
        if failed:
            return reason
    return None


def run_package_device_gates(budget_seconds: float) -> int:
    for package in DEVICE_PACKAGES:
        label = f"{package.removeprefix('unitas-')}-device"
        rc, _ = run_step(
            label,
            ("cargo", "test", "-p", package, "--features", "device"),
            budget_seconds,
        )
        if rc != 0:
            return rc
    for package in DEVICE_PACKAGES:
        label = f"{package.removeprefix('unitas-')}-clippy"
        rc, _ = run_step(
            label,
            (
                "cargo",
                "clippy",
                "-p",
                package,
                "--features",
                "device",
                "--all-targets",
                "--",
                "-D",
                "clippy::correctness",
                "-D",
                "clippy::suspicious",
            ),
            budget_seconds,
        )
        if rc != 0:
            return rc
    return 0


def prepare_actual_capture(actual_capture: Path | None, budget_seconds: float) -> tuple[int, Path]:
    if actual_capture is not None:
        return 0, actual_capture
    rc, _ = run_step(
        "capture-gpu-frame",
        (str(PLAYER_BINARY), "--capture-reference", str(ACTUAL_CAPTURE_PATH)),
        budget_seconds,
    )
    return rc, ACTUAL_CAPTURE_PATH


def run_device_lane(
    actual_capture: Path | None = None,
    budget_seconds: float = DEFAULT_BUDGET_SECONDS,
) -> int:
    sys.stdout.write(metadata_text() + "\n")
    rc, _ = run_step(
        "build-player",
        (
            "cargo",
            "build",
            "-p",
            "unitas-build",
            "--bin",
            "unitas-player",
            "--features",
            "device",
        ),
        BUILD_BUDGET_SECONDS,
    )
    if rc != 0:
        return rc

    rc, actual = prepare_actual_capture(actual_capture, budget_seconds)
    if rc != 0:
        return rc

    ok, _mean, _max, summary = compare_bmp_files(
        actual,
        DEFAULT_REFERENCE_BMP,
        DEFAULT_TOLERANCE,
    )
    if not ok:
        sys.stderr.write(f"FAIL verify-device (reference-compare): {summary} ({POLICY_CITATION})\n")
        return 1
    sys.stdout.write(f"OK verify-device reference-compare: {summary}\n")

    rc, output = run_step(
        "device-verify",
        (
            str(PLAYER_BINARY),
            "--verify-device",
            str(DEFAULT_REFERENCE_BMP),
            "--frames",
            str(DEVICE_FRAMES),
        ),
        budget_seconds,
    )
    if rc != 0:
        return rc
    reason = device_failure_reason(parse_device_report(output))
    if reason is not None:
        sys.stderr.write(f"FAIL verify-device (device-verify): {reason} ({POLICY_CITATION})\n")
        return 1
    sys.stdout.write(
        "OK verify-device device-verify: the window presented frames, the GPU frame matched "
        "the approved reference within tolerance, the default audio device played the tone, "
        "and the synthetic input events moved the sample scene\n"
    )

    rc = run_package_device_gates(budget_seconds)
    if rc == 0:
        sys.stdout.write("OK verify-device\n")
    return rc


def run_compare_command(args: Sequence[str]) -> int:
    if len(args) not in (COMPARE_ARG_COUNT, COMPARE_WITH_TOLERANCE_ARG_COUNT):
        sys.stderr.write(help_text() + "\n")
        return 2
    actual = Path(args[1])
    reference = Path(args[2])
    tolerance = DEFAULT_TOLERANCE
    if len(args) == COMPARE_WITH_TOLERANCE_ARG_COUNT:
        if args[3] != "--tolerance":
            sys.stderr.write(help_text() + "\n")
            return 2
        try:
            tolerance = float(args[4])
        except ValueError:
            sys.stderr.write(f"invalid tolerance: {args[4]}\n")
            return 2
    ok, _mean, _max, summary = compare_bmp_files(actual, reference, tolerance)
    if not ok:
        sys.stderr.write(f"FAIL verify-device (compare): {summary} ({POLICY_CITATION})\n")
        return 1
    sys.stdout.write(f"OK verify-device (compare): {summary}\n")
    return 0


def main(argv: Sequence[str]) -> int:
    args = list(argv[1:])
    if not args:
        return run_device_lane()
    if args in (["--help"], ["-h"]):
        sys.stdout.write(help_text() + "\n")
        return 0
    if args == ["--metadata"]:
        sys.stdout.write(metadata_text() + "\n")
        return 0
    if args[0] == "--actual" and len(args) == ACTUAL_ARG_COUNT:
        return run_device_lane(actual_capture=Path(args[1]))
    if args[0] == "--compare":
        return run_compare_command(args)
    sys.stderr.write(help_text() + "\n")
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
