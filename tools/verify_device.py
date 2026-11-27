from __future__ import annotations

import contextlib
import json
import math
import queue
import re
import struct
import subprocess
import sys
import threading
import time
import wave
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
DEVICE_FRAMES = 240
COMPARE_ARG_COUNT = 3
COMPARE_WITH_TOLERANCE_ARG_COUNT = 5
ACTUAL_ARG_COUNT = 2
MIN_SUSTAINED_FPS = 30.0
CONTROL_STDIO_KEY_W = ord("w")
CONTROL_STDIO_KEY_D = ord("d")
CONTROL_STDIO_CHUNK_FRAMES = 30
CONTROL_STDIO_MAX_FRAMES = 300
CONTROL_STDIO_BUDGET_SECONDS = 90.0
CONTROL_STDIO_STOP_TIMEOUT_SECONDS = 5.0
CONTROL_STATE_FIELDS = ("frames", "script_host")
CANVAS_TEXT_SEPARATOR = " | "
GAME_SCORE_PATTERN = re.compile(r"Pickup collected score (\d+)")
GAME_CHIME_PATTERN = re.compile(r"collect chime played pickup_frame=(\d+)")
ACTUAL_CAPTURE_PATH = REPO_ROOT / "target" / "device-lane" / "actual_gpu_frame.bmp"
DEVICE_AUDIO_WAV = REPO_ROOT / "target" / "device-lane" / "device_audio.wav"
PCM_SAMPLE_WIDTH_BYTES = 2
GAME_FRAME_RATE = 60
CHIME_FREQUENCY_HZ = 875.0
CHIME_WINDOW_SECONDS = 0.05
CHIME_LEAD_IN_SECONDS = 0.5
CHIME_SILENCE_SECONDS = 0.25
CHIME_FLOOR_AMPLITUDE = 0.005
PLAYER_BINARY = REPO_ROOT / "target" / "debug" / "unitas-player"
POLICY_CITATION = "docs/ralph-workflow-policy/verification-policy.md § Gate lanes"
TIMEOUT_CITATION = "docs/ralph-workflow-policy/verification-policy.md § Default requirements"
DEVICE_PACKAGES: tuple[str, ...] = (
    "unitas-audio",
    "unitas-runtime",
    "unitas-build",
)

LANE_METADATA: tuple[tuple[str, str], ...] = (
    ("owner", "presentation and player"),
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
    ("canvas", r"canvas=(true|false)"),
    ("canvas_texts", r'canvas_texts="([^"]*)"'),
    ("shadow", r"shadow=(true|false)"),
)

SUSTAINED_FPS_PATTERN = re.compile(r"^sustained_fps=([0-9]+(?:\.[0-9]+)?)", re.MULTILINE)


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
        if match is not None:
            report[name] = match.group(1)
    return report


def parse_sustained_fps(output: str) -> float | None:
    match = SUSTAINED_FPS_PATTERN.search(output)
    if match is None:
        return None
    return float(match.group(1))


def sustained_fps_below_floor_reason(sustained_fps: float) -> str | None:
    if sustained_fps < MIN_SUSTAINED_FPS:
        return (
            f"sustained {sustained_fps:.1f} FPS is below the interactive floor "
            f"{MIN_SUSTAINED_FPS:.1f}"
        )
    return None


def sustained_fps_measurement(output: str) -> tuple[str, str | None]:
    sustained_fps = parse_sustained_fps(output)
    if sustained_fps is None:
        return "unknown", "the window reported no sustained FPS measurement"
    return f"{sustained_fps:.1f}", sustained_fps_below_floor_reason(sustained_fps)


def device_failure_reason(report: dict[str, str] | None) -> str | None:
    if report is None:
        return "the player reported no device verification line"
    missing = [name for name, _pattern in DEVICE_REPORT_FIELDS if name not in report]
    if missing:
        names = ", ".join(missing)
        return (
            f"the device verification line lacks fields: {names}; "
            "contract C3 requires size, frames_presented, mean_error, tolerance, "
            "audio_active, audio_frames, canvas, canvas_texts, and shadow"
        )
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
            f"the mixed audio pushed {audio_frames} frames to the audio device; "
            "at least 1 is required",
        ),
        (
            report["canvas"] != "true",
            "the uGUI canvas did not render on the windowed surface (canvas=false); "
            "the canvas content must be part of the presented frame, not only of the capture",
        ),
        (
            not report["canvas_texts"].strip(),
            "the uGUI canvas carried no text (canvas_texts is empty); "
            "the game's readout must be part of the presented frame per contract C3",
        ),
        (
            report["shadow"] != "true",
            "the windowed frame was presented with shadows disabled (shadow=false); "
            "the bundled player must resolve the project quality settings for its target group",
        ),
    )
    for failed, reason in checks:
        if failed:
            return reason
    return None


def canvas_texts(report: dict[str, str] | None) -> list[str]:
    if report is None:
        return []
    return [text for text in report.get("canvas_texts", "").split(CANVAS_TEXT_SEPARATOR) if text]


def game_score(lines: Sequence[str]) -> int | None:
    score: int | None = None
    for line in lines:
        match = GAME_SCORE_PATTERN.search(line)
        if match is None:
            continue
        value = int(match.group(1))
        if score is None or value > score:
            score = value
    return score


def game_progress_reason(lines: Sequence[str]) -> str | None:
    score = game_score(lines)
    if score is None:
        return (
            "the player's log output carries no collected pickup; "
            "the synthetic input events did not reach the sample game"
        )
    if score < 1:
        return f"the sample game reports a score of {score}; at least 1 is required"
    return None


def pickup_frames(lines: Sequence[str]) -> list[int]:
    frames: list[int] = []
    for line in lines:
        match = GAME_CHIME_PATTERN.search(line)
        if match is not None:
            frames.append(int(match.group(1)))
    return frames


def read_wav_mono(path: Path) -> tuple[int, list[float]]:
    with wave.open(str(path), "rb") as handle:
        channels = handle.getnchannels()
        sample_rate = handle.getframerate()
        width = handle.getsampwidth()
        raw = handle.readframes(handle.getnframes())
    if width != PCM_SAMPLE_WIDTH_BYTES:
        message = f"unsupported sample width {width} bytes"
        raise ValueError(message)
    count = len(raw) // PCM_SAMPLE_WIDTH_BYTES
    values = struct.unpack(f"<{count}h", raw[: count * PCM_SAMPLE_WIDTH_BYTES])
    if channels == 1:
        return sample_rate, [value / 32768.0 for value in values]
    mono: list[float] = []
    for index in range(count // channels):
        start = index * channels
        mono.append(sum(values[start : start + channels]) / (channels * 32768.0))
    return sample_rate, mono


def window_energy(samples: Sequence[float]) -> float:
    if not samples:
        return 0.0
    return math.sqrt(sum(sample * sample for sample in samples) / len(samples))


def chime_window_finding(samples: Sequence[float], sample_rate: int) -> tuple[str | None, str]:
    offset = round(CHIME_LEAD_IN_SECONDS * sample_rate)
    window = round(CHIME_WINDOW_SECONDS * sample_rate)
    if offset + window > len(samples):
        return (
            f"the device audio capture holds {len(samples)} samples at {sample_rate} Hz; "
            f"the chime window [{offset}, {offset + window}) lies outside the capture",
            "",
        )
    silence = round(CHIME_SILENCE_SECONDS * sample_rate)
    silence_amplitude = window_energy(samples[:silence])
    amplitude = window_energy(samples[offset : offset + window])
    summary = (
        f"the audio energy in the window after the injected pickup "
        f"is {amplitude:.5f} against the silence floor {CHIME_FLOOR_AMPLITUDE:.5f}"
    )
    if amplitude <= CHIME_FLOOR_AMPLITUDE:
        return (f"the mixed audio after the pickup carries no collect chime: {summary}", "")
    if silence_amplitude > CHIME_FLOOR_AMPLITUDE:
        return (
            f"the captured device stream carries audio before the injected "
            f"pickup: {silence_amplitude:.5f} against the silence floor "
            f"{CHIME_FLOOR_AMPLITUDE:.5f}",
            "",
        )
    return (None, summary)


def chime_capture_reason(wav_path: Path) -> tuple[str | None, str]:
    if not wav_path.is_file():
        return (f"the device lane wrote no audio capture at '{wav_path}'", "")
    try:
        sample_rate, samples = read_wav_mono(wav_path)
    except (OSError, ValueError, wave.Error) as error:
        return (f"the device audio capture '{wav_path}' is unreadable: {error}", "")
    if sample_rate <= 0 or not samples:
        return (f"the device audio capture '{wav_path}' carries no samples", "")
    return chime_window_finding(samples, sample_rate)


def start_control_process() -> subprocess.Popen[str]:
    return subprocess.Popen(
        [str(PLAYER_BINARY), "--control-stdio"],
        cwd=REPO_ROOT,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )


def pump_control_responses(
    process: subprocess.Popen[str],
    responses: queue.Queue[str | None],
) -> None:
    if process.stdout is not None:
        for line in process.stdout:
            responses.put(line)
    responses.put(None)


def pump_control_errors(process: subprocess.Popen[str], errors: list[str]) -> None:
    if process.stderr is not None:
        errors.extend(process.stderr)


def stop_control_process(process: subprocess.Popen[str]) -> None:
    if process.stdin is not None:
        with contextlib.suppress(OSError):
            process.stdin.close()
    if process.poll() is None:
        try:
            process.wait(timeout=CONTROL_STDIO_STOP_TIMEOUT_SECONDS)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


def control_request(
    process: subprocess.Popen[str],
    responses: queue.Queue[str | None],
    deadline: float,
    command: dict[str, object],
) -> dict[str, object]:
    if process.stdin is None:
        message = "control-stdio stdin is closed"
        raise ControlCheckError(message)
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        message = "the control-stdio check exceeded its time budget"
        raise ControlCheckError(message)
    try:
        process.stdin.write(json.dumps(command) + "\n")
        process.stdin.flush()
        line = responses.get(timeout=remaining)
    except OSError as error:
        message = f"control-stdio write failed: {error}"
        raise ControlCheckError(message) from error
    except queue.Empty as error:
        message = "control-stdio timed out without a response"
        raise ControlCheckError(message) from error
    if line is None:
        message = "control-stdio closed the connection without a response"
        raise ControlCheckError(message)
    try:
        payload = json.loads(line)
    except ValueError as error:
        message = f"control-stdio returned invalid JSON: {error}"
        raise ControlCheckError(message) from error
    if not isinstance(payload, dict):
        message = "control-stdio returned a non-object response"
        raise ControlCheckError(message)
    if payload.get("ok") is not True:
        detail = payload.get("error", "unknown error")
        message = f"control-stdio command {command['cmd']} failed: {detail}"
        raise ControlCheckError(message)
    return payload


def control_state(payload: dict[str, object]) -> int:
    missing = [field for field in CONTROL_STATE_FIELDS if field not in payload]
    if missing:
        names = ", ".join(missing)
        message = f"control-stdio state response lacks contract C3 fields: {names}"
        raise ControlCheckError(message)
    frames = payload["frames"]
    if not isinstance(frames, int) or isinstance(frames, bool):
        message = "control-stdio state response frames is not an integer"
        raise ControlCheckError(message)
    script_host = payload["script_host"]
    if script_host != "coreclr":
        message = (
            "control-stdio state response did not host the game's scripts on CoreCLR "
            f"(script_host={script_host!r})"
        )
        raise ControlCheckError(message)
    return frames


def drive_control_session(
    process: subprocess.Popen[str],
    responses: queue.Queue[str | None],
    deadline: float,
    errors: list[str],
) -> int:
    control_request(process, responses, deadline, {"cmd": "play"})
    control_state(control_request(process, responses, deadline, {"cmd": "state"}))
    control_request(
        process,
        responses,
        deadline,
        {
            "cmd": "inject_input",
            "events": [
                {"kind": "key", "key": CONTROL_STDIO_KEY_W, "pressed": True},
                {"kind": "key", "key": CONTROL_STDIO_KEY_D, "pressed": True},
            ],
        },
    )
    frames = 0
    while frames < CONTROL_STDIO_MAX_FRAMES:
        control_request(
            process,
            responses,
            deadline,
            {"cmd": "step", "frames": CONTROL_STDIO_CHUNK_FRAMES},
        )
        frames += CONTROL_STDIO_CHUNK_FRAMES
        if game_progress_reason(errors) is None:
            return game_score(errors) or 0
    message = (
        f"drove the ball forward for {frames} frames but the player logged no collected pickup"
    )
    raise ControlCheckError(message)


def control_stdio_check(budget_seconds: float) -> str | None:
    deadline = time.monotonic() + min(budget_seconds, CONTROL_STDIO_BUDGET_SECONDS)
    responses: queue.Queue[str | None] = queue.Queue()
    errors: list[str] = []
    process = start_control_process()
    threading.Thread(
        target=pump_control_responses,
        args=(process, responses),
        daemon=True,
    ).start()
    threading.Thread(
        target=pump_control_errors,
        args=(process, errors),
        daemon=True,
    ).start()
    try:
        drive_control_session(process, responses, deadline, errors)
    except ControlCheckError as error:
        return str(error)
    except OSError as error:
        return f"control-stdio process failed: {error}"
    finally:
        stop_control_process(process)
    return None


class ControlCheckError(Exception):
    pass


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


def device_lane_failure(
    report: dict[str, str] | None,
    output: str,
    fps_reason: str | None,
    budget_seconds: float,
) -> tuple[str, str | None, str]:
    lines = output.splitlines()
    reason = device_failure_reason(report) or game_progress_reason(lines)
    stage = "device-verify"
    if reason is None:
        reason = fps_reason
    chime_reason: str | None = "the player's log output carries no collected pickup chime frame"
    chime_summary = ""
    chime_frames = pickup_frames(lines)
    if chime_frames:
        chime_reason, chime_summary = chime_capture_reason(DEVICE_AUDIO_WAV)
    if reason is None:
        stage = "device-audio"
        reason = chime_reason
    if reason is None:
        stage = "control-stdio"
        reason = control_stdio_check(budget_seconds)
    return stage, reason, chime_summary


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
    report = parse_device_report(output)
    sustained_fps_text, fps_reason = sustained_fps_measurement(output)
    stage, reason, chime_summary = device_lane_failure(report, output, fps_reason, budget_seconds)
    if reason is not None:
        sys.stderr.write(f"FAIL verify-device ({stage}): {reason} ({POLICY_CITATION})\n")
        return 1
    canvas_summary = ""
    if report is not None:
        canvas_summary = (
            f" Canvas text: {'; '.join(canvas_texts(report))}, "
            f"canvas on screen={report['canvas']}, shadows on screen={report['shadow']}."
        )
    sys.stdout.write(
        "OK verify-device device-verify: the window presented frames, the GPU frame matched "
        "the approved reference within tolerance, the default audio device played the mixed audio, "
        "the uGUI canvas and the project shadows were part of the presented frame, "
        "and the synthetic input events moved the sample scene. Sustained frame rate: "
        f"{sustained_fps_text} FPS (interactive floor {MIN_SUSTAINED_FPS:.1f}){canvas_summary}\n"
        "OK verify-device control-stdio: the packaged player hosted the game's scripts on "
        "CoreCLR and the scripted input drove the ball into a pickup\n"
        f"OK verify-device device-audio: the captured device stream carries the collect "
        f"chime: {chime_summary}\n"
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
