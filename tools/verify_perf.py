from __future__ import annotations

import ctypes
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence

REPO_ROOT = Path(__file__).resolve().parent.parent
RUNTIME_LIBRARY = REPO_ROOT / "target" / "debug" / "libunitas_runtime.dylib"
PLAYER_BINARY = REPO_ROOT / "target" / "debug" / "unitas-player"
POLICY_CITATION = "docs/ralph-workflow-policy/verification-policy.md § Gate lanes"
STEP_BUDGET_SECONDS = 300.0
LANE_BUDGET_SECONDS = 600.0
IMPORT_MEASUREMENT = "import_throughput"
IMPORT_REIMPORT_CEILING_MS = 8.4
SCRIPTING_CALL_COST = "scripting_call_cost"
PLAYERLOOP_FRAME_ALLOCATIONS = "playerloop_frame_allocations"
PLAYER_STARTUP = "player_startup"
PLAYER_STARTUP_CEILING_MS = 3000.0
PLAYER_STARTUP_WIDTH = 320
PLAYER_STARTUP_HEIGHT = 240
AUDIO_CALLBACK_HEADROOM = "audio_callback_headroom"
AUDIO_CALLBACK_SAMPLE_RATE = 48_000
AUDIO_CALLBACK_BUFFER_FRAMES = 512
AUDIO_CALLBACK_CHANNELS = 2
AUDIO_CALLBACK_BYTES_PER_FRAME = 8
AUDIO_CALLBACK_VOICE_COUNT = 8
AUDIO_CALLBACK_WARMUP_PASSES = 10
AUDIO_CALLBACK_MEASURED_PASSES = 100
AUDIO_CALLBACK_CEILING_MS = round(
    (AUDIO_CALLBACK_BUFFER_FRAMES * 1000.0) / AUDIO_CALLBACK_SAMPLE_RATE,
    3,
)
BRIDGE_TESTS = "bridge/Unitas.Bridge.Tests/bin/Release/net8.0/Unitas.Bridge.Tests"
EDITOR_TESTS = "editor/Unitas.Editor.Tests/bin/Release/net8.0/Unitas.Editor.Tests.dll"
EDITOR_MEASUREMENT_CEILINGS_MS: dict[str, float] = {
    "editor_startup": 5000.0,
    "editor_project_open": 500.0,
    "editor_hierarchy_search_100k": 15000.0,
    "editor_property_edit_undo": 500.0,
    "editor_mcp_round_trip": 100.0,
}

LANE_METADATA: tuple[tuple[str, str], ...] = (
    ("owner", "content and simulation"),
    ("trigger", "every milestone touching import, scene load, physics, or animation"),
    ("lane", "make verify-perf"),
    ("evidence", "per-measurement OK/FAIL lines with value_ms against ceiling_ms"),
    ("review", "when a perf measurement is added or a ceiling moves"),
)

MEASUREMENTS: tuple[tuple[str, tuple[str, ...]], ...] = (
    (
        IMPORT_MEASUREMENT,
        (
            "cargo",
            "test",
            "-p",
            "unitas-asset",
            "--test",
            "asset_import_tests",
            "perf_import",
            "--",
            "--nocapture",
        ),
    ),
    (
        "scene_load",
        (
            "cargo",
            "test",
            "-p",
            "unitas-asset",
            "--test",
            "asset_import_tests",
            "perf_scene_load",
            "--",
            "--ignored",
            "--nocapture",
        ),
    ),
    (
        "physics_step",
        (
            "cargo",
            "test",
            "-p",
            "unitas-physics",
            "--test",
            "simulation_tests",
            "perf_step",
            "--",
            "--ignored",
            "--nocapture",
        ),
    ),
    (
        "particle_eval",
        (
            "cargo",
            "test",
            "-p",
            "unitas-physics",
            "--test",
            "simulation_tests",
            "perf_particles",
            "--",
            "--ignored",
            "--nocapture",
        ),
    ),
    (
        "animation_eval",
        (
            "cargo",
            "test",
            "-p",
            "unitas-animation",
            "--test",
            "animation_tests",
            "perf_eval",
            "--",
            "--ignored",
            "--nocapture",
        ),
    ),
    (
        "frame_loop_100k",
        (
            "cargo",
            "test",
            "-p",
            "unitas-runtime",
            "--test",
            "perf_frame_loop",
            "--",
            "--ignored",
            "--nocapture",
        ),
    ),
    (
        SCRIPTING_CALL_COST,
        (
            BRIDGE_TESTS,
            "--perf",
        ),
    ),
    (
        PLAYERLOOP_FRAME_ALLOCATIONS,
        (
            BRIDGE_TESTS,
            "--perf",
        ),
    ),
    (
        PLAYER_STARTUP,
        (
            sys.executable,
            str(Path(__file__).resolve()),
            "--measure-player-startup",
        ),
    ),
    (
        AUDIO_CALLBACK_HEADROOM,
        (
            sys.executable,
            str(Path(__file__).resolve()),
            "--measure-audio-callback-headroom",
        ),
    ),
    (
        "editor_startup",
        (
            "dotnet",
            EDITOR_TESTS,
            "--perf",
            "editor_startup",
        ),
    ),
    (
        "editor_project_open",
        (
            "dotnet",
            EDITOR_TESTS,
            "--perf",
            "editor_project_open",
        ),
    ),
    (
        "editor_hierarchy_search_100k",
        (
            "dotnet",
            EDITOR_TESTS,
            "--perf",
            "editor_hierarchy_search_100k",
        ),
    ),
    (
        "editor_property_edit_undo",
        (
            "dotnet",
            EDITOR_TESTS,
            "--perf",
            "editor_property_edit_undo",
        ),
    ),
    (
        "editor_mcp_round_trip",
        (
            "dotnet",
            EDITOR_TESTS,
            "--perf",
            "editor_mcp_round_trip",
        ),
    ),
)

BYTES_MEASUREMENTS = frozenset({PLAYERLOOP_FRAME_ALLOCATIONS})
MEASUREMENT_PRECISION: dict[str, int] = {SCRIPTING_CALL_COST: 6}

MEASUREMENT_LINE = re.compile(r"measurement=(\S+) value_ms=([0-9.]+) ceiling_ms=([0-9.]+)")
EDITOR_MEASUREMENT_LINE = re.compile(
    r"^(?P<name>[a-z][a-z0-9_]*)\t(?P<value>[0-9.]+)\r?$", re.MULTILINE
)
MEASUREMENT_BYTES_LINE = re.compile(
    r"measurement=(\S+) value_bytes=([0-9.]+) ceiling_bytes=([0-9.]+)"
)
IMPORT_DURATION = re.compile(r"reimport ([0-9.]+)(ns|µs|ms|s)\b")
DURATION_FACTOR_MS: dict[str, float] = {"ns": 1e-6, "µs": 1e-3, "ms": 1.0, "s": 1e3}

BUILD_STEPS: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("runtime-build", ("cargo", "build", "-p", "unitas-runtime")),
    (
        "player-build",
        ("cargo", "build", "-p", "unitas-build", "--bin", "unitas-player", "--features", "device"),
    ),
    ("bridge-build", ("dotnet", "build", "bridge/Unitas.sln", "--configuration", "Release")),
)


def help_text() -> str:
    metadata_lines = [f"  {key}: {value}" for key, value in LANE_METADATA]
    return "\n".join(
        [
            "usage: verify_perf.py [--help] [--metadata]",
            "Runs the fifteen perf measurements (import throughput, scene load, physics",
            "step, particle eval, animation eval, 100k GameObject frame loop, scripting",
            "call cost, PlayerLoop frame allocations, player startup, audio callback",
            "headroom, editor startup, editor project open, editor hierarchy search at the",
            "scale floor, editor property edit and undo, and editor MCP round trip) and",
            "fails hard when any value breaches its ceiling.",
            "lane metadata:",
            *metadata_lines,
        ]
    )


def metadata_text() -> str:
    return "\n".join(f"{key}: {value}" for key, value in LANE_METADATA)


def parse_editor_measurement(name: str, output: str) -> tuple[float, float] | None:
    for match in EDITOR_MEASUREMENT_LINE.finditer(output):
        if match.group("name") == name:
            return (float(match.group("value")), EDITOR_MEASUREMENT_CEILINGS_MS[name])
    return None


def parse_measurement(name: str, output: str) -> tuple[float, float] | None:
    if name in EDITOR_MEASUREMENT_CEILINGS_MS:
        return parse_editor_measurement(name, output)
    if name in BYTES_MEASUREMENTS:
        return parse_bytes_measurement(output)
    return parse_value_measurement(name, output)


def parse_bytes_measurement(output: str) -> tuple[float, float] | None:
    bytes_match = MEASUREMENT_BYTES_LINE.search(output)
    if bytes_match is None:
        return None
    return (float(bytes_match.group(2)), float(bytes_match.group(3)))


def parse_value_measurement(name: str, output: str) -> tuple[float, float] | None:
    if name == IMPORT_MEASUREMENT:
        duration = IMPORT_DURATION.search(output)
        if duration is None:
            return None
        value_ms = float(duration.group(1)) * DURATION_FACTOR_MS[duration.group(2)]
        return (value_ms, IMPORT_REIMPORT_CEILING_MS)
    match = MEASUREMENT_LINE.search(output)
    if match is None:
        return None
    return (float(match.group(2)), float(match.group(3)))


def run_step(
    name: str,
    command: Sequence[str],
    budget_seconds: float,
    env: dict[str, str],
) -> int:
    started = time.monotonic()
    sys.stdout.write(f"step {name}: {' '.join(command)}\n")
    try:
        completed = subprocess.run(
            list(command),
            cwd=REPO_ROOT,
            env=env,
            check=False,
            timeout=budget_seconds,
            capture_output=True,
            text=True,
            errors="replace",
        )
    except subprocess.TimeoutExpired:
        sys.stderr.write(
            f"FAIL verify-perf ({name}): exceeded the {budget_seconds:.0f}s step budget "
            f"({POLICY_CITATION})\n"
        )
        return 1
    elapsed = time.monotonic() - started
    if completed.returncode != 0:
        sys.stderr.write(
            f"FAIL verify-perf ({name}): the build step exited {completed.returncode} "
            f"({POLICY_CITATION})\n"
        )
        output = completed.stdout + completed.stderr
        sys.stderr.write(output if output.endswith("\n") else output + "\n")
        return 1
    sys.stdout.write(f"step {name}: exit {completed.returncode} in {elapsed:.1f}s\n")
    return 0


def run_measurement(
    name: str,
    command: Sequence[str],
    budget_seconds: float,
    env: dict[str, str],
) -> tuple[float, float] | None:
    started = time.monotonic()
    try:
        completed = subprocess.run(
            list(command),
            cwd=REPO_ROOT,
            env=env,
            check=False,
            timeout=budget_seconds,
            capture_output=True,
            text=True,
            errors="replace",
        )
    except subprocess.TimeoutExpired:
        elapsed = time.monotonic() - started
        sys.stderr.write(
            f"FAIL verify-perf ({name}): timed out after {elapsed:.2f}s "
            f"(budget {budget_seconds:.0f}s); a slow measurement is a defect to "
            f"diagnose, never a budget to raise ({POLICY_CITATION})\n"
        )
        return None
    output = completed.stdout + completed.stderr
    if completed.returncode != 0:
        sys.stderr.write(
            f"FAIL verify-perf ({name}): the measurement test exited "
            f"{completed.returncode} ({POLICY_CITATION})\n"
        )
        sys.stderr.write(output if output.endswith("\n") else output + "\n")
        return None
    measurement = parse_measurement(name, output)
    if measurement is None:
        sys.stderr.write(
            f"FAIL verify-perf ({name}): the test printed no parseable "
            f"measurement line ({POLICY_CITATION})\n"
        )
    return measurement


def resolve_dotnet_root() -> Path | None:
    dotnet_executable = shutil.which("dotnet")
    if dotnet_executable is not None:
        resolved_parent = Path(dotnet_executable).resolve().parent
        if (resolved_parent / "host" / "fxr").is_dir():
            return resolved_parent
    home_dotnet = Path.home() / ".dotnet"
    if home_dotnet.is_dir():
        return home_dotnet
    return None


def measure_player_startup_command() -> int:
    with tempfile.TemporaryDirectory(prefix="unitas_perf_player_startup_") as temp_dir:
        bundle_dir = Path(temp_dir) / "bundle"
        stage_completed = subprocess.run(
            [str(PLAYER_BINARY), "--sample", str(bundle_dir), "--frames", "1"],
            cwd=REPO_ROOT,
            check=False,
            capture_output=True,
            text=True,
            errors="replace",
        )
        if stage_completed.returncode != 0:
            sys.stderr.write(stage_completed.stdout + stage_completed.stderr)
            return 1
        started = time.monotonic()
        launch_completed = subprocess.run(
            [
                str(PLAYER_BINARY),
                str(bundle_dir),
                "--windowed",
                "--width",
                str(PLAYER_STARTUP_WIDTH),
                "--height",
                str(PLAYER_STARTUP_HEIGHT),
                "--frames",
                "1",
            ],
            cwd=REPO_ROOT,
            check=False,
            capture_output=True,
            text=True,
            errors="replace",
        )
        elapsed_ms = (time.monotonic() - started) * 1000.0
        output = launch_completed.stdout + launch_completed.stderr
        if launch_completed.returncode != 0 or "frames_presented=1" not in output:
            sys.stderr.write(output if output.endswith("\n") else output + "\n")
            return 1
        sys.stdout.write(
            f"measurement={PLAYER_STARTUP} "
            f"value_ms={elapsed_ms:.3f} "
            f"ceiling_ms={PLAYER_STARTUP_CEILING_MS:.3f}\n"
        )
        return 0


def configure_audio_abi(library: ctypes.CDLL) -> None:
    library.unitas_runtime_new.restype = ctypes.c_uint64
    library.unitas_runtime_destroy.argtypes = [ctypes.c_uint64]
    library.unitas_runtime_destroy.restype = ctypes.c_int32
    library.unitas_runtime_audio_create.argtypes = [ctypes.c_uint64, ctypes.c_uint64]
    library.unitas_runtime_audio_create.restype = ctypes.c_uint64
    library.unitas_runtime_audio_register_clip_samples.argtypes = [
        ctypes.c_uint64,
        ctypes.c_uint32,
        ctypes.c_uint32,
        ctypes.c_uint64,
        ctypes.c_uint64,
    ]
    library.unitas_runtime_audio_register_clip_samples.restype = ctypes.c_uint64
    library.unitas_runtime_audio_set_clip.argtypes = [
        ctypes.c_uint64,
        ctypes.c_uint64,
        ctypes.c_uint64,
    ]
    library.unitas_runtime_audio_set_clip.restype = ctypes.c_int32
    library.unitas_runtime_audio_set_loop.argtypes = [
        ctypes.c_uint64,
        ctypes.c_uint64,
        ctypes.c_uint32,
    ]
    library.unitas_runtime_audio_set_loop.restype = ctypes.c_int32
    library.unitas_runtime_audio_play.argtypes = [
        ctypes.c_uint64,
        ctypes.c_uint64,
        ctypes.c_uint32,
    ]
    library.unitas_runtime_audio_play.restype = ctypes.c_int32
    library.unitas_runtime_path_buffer.argtypes = [ctypes.c_uint64, ctypes.c_uint64]
    library.unitas_runtime_path_buffer.restype = ctypes.c_uint64
    library.unitas_runtime_audio_mix.argtypes = [
        ctypes.c_uint64,
        ctypes.c_uint64,
        ctypes.c_uint64,
    ]
    library.unitas_runtime_audio_mix.restype = ctypes.c_uint64


def seed_audio_callback_voices(library: ctypes.CDLL, handle: int, clip_id: int) -> bool:
    for index in range(1, AUDIO_CALLBACK_VOICE_COUNT + 1):
        entity = int(library.unitas_runtime_audio_create(handle, index))
        if entity == 0:
            return False
        if int(library.unitas_runtime_audio_set_clip(handle, entity, clip_id)) != 0:
            return False
        if int(library.unitas_runtime_audio_set_loop(handle, entity, 1)) != 0:
            return False
        if int(library.unitas_runtime_audio_play(handle, entity, 0)) != 0:
            return False
    return True


def run_audio_callback_passes(
    library: ctypes.CDLL,
    handle: int,
    out_buffer: int,
    passes: int,
) -> bool:
    for _ in range(passes):
        mixed = int(
            library.unitas_runtime_audio_mix(
                handle,
                AUDIO_CALLBACK_BUFFER_FRAMES,
                out_buffer,
            )
        )
        if mixed != AUDIO_CALLBACK_BUFFER_FRAMES:
            return False
    return True


def measure_audio_callback_ms(library: ctypes.CDLL, handle: int) -> float | None:
    clip_id = int(
        library.unitas_runtime_audio_register_clip_samples(
            handle,
            AUDIO_CALLBACK_CHANNELS,
            AUDIO_CALLBACK_SAMPLE_RATE,
            AUDIO_CALLBACK_SAMPLE_RATE,
            0,
        )
    )
    if clip_id == 0 or not seed_audio_callback_voices(library, handle, clip_id):
        return None
    byte_len = AUDIO_CALLBACK_BUFFER_FRAMES * AUDIO_CALLBACK_BYTES_PER_FRAME
    out_buffer = int(library.unitas_runtime_path_buffer(handle, byte_len))
    if out_buffer == 0:
        return None
    if not run_audio_callback_passes(library, handle, out_buffer, AUDIO_CALLBACK_WARMUP_PASSES):
        return None
    started = time.monotonic()
    if not run_audio_callback_passes(library, handle, out_buffer, AUDIO_CALLBACK_MEASURED_PASSES):
        return None
    return ((time.monotonic() - started) * 1000.0) / AUDIO_CALLBACK_MEASURED_PASSES


def measure_audio_callback_headroom_command() -> int:
    if not RUNTIME_LIBRARY.is_file():
        sys.stderr.write(f"runtime library missing at {RUNTIME_LIBRARY}\n")
        return 1
    library = ctypes.CDLL(str(RUNTIME_LIBRARY))
    configure_audio_abi(library)
    handle = int(library.unitas_runtime_new())
    if handle == 0:
        return 1
    try:
        elapsed_ms = measure_audio_callback_ms(library, handle)
    finally:
        library.unitas_runtime_destroy(handle)
    if elapsed_ms is None:
        return 1
    sys.stdout.write(
        f"measurement={AUDIO_CALLBACK_HEADROOM} "
        f"value_ms={elapsed_ms:.3f} "
        f"ceiling_ms={AUDIO_CALLBACK_CEILING_MS:.3f}\n"
    )
    return 0


def run_perf_lane() -> int:
    sys.stdout.write(metadata_text() + "\n")
    env = dict(os.environ)
    env["UNITAS_RUNTIME_LIBRARY"] = str(RUNTIME_LIBRARY)
    dotnet_root = resolve_dotnet_root()
    if dotnet_root is not None:
        env["DOTNET_ROOT"] = str(dotnet_root)
    lane_started = time.monotonic()
    for name, command in BUILD_STEPS:
        if run_step(name, command, STEP_BUDGET_SECONDS, env) != 0:
            sys.stderr.write(
                f"FAIL verify-perf: the {name} step failed; a broken build is a lane "
                f"failure ({POLICY_CITATION})\n"
            )
            return 1
    failures = 0
    for name, command in MEASUREMENTS:
        measurement = run_measurement(name, command, STEP_BUDGET_SECONDS, env)
        if measurement is None:
            failures += 1
            continue
        value, ceiling = measurement
        unit = "bytes" if name in BYTES_MEASUREMENTS else "ms"
        precision = MEASUREMENT_PRECISION.get(name, 3)
        if value > ceiling:
            sys.stderr.write(
                f"FAIL verify-perf ({name}): value_{unit}={value:.{precision}f} breaches "
                f"ceiling_{unit}={ceiling:.{precision}f} ({POLICY_CITATION})\n"
            )
            failures += 1
            continue
        sys.stdout.write(
            f"OK verify-perf ({name}): value_{unit}={value:.{precision}f} "
            f"ceiling_{unit}={ceiling:.{precision}f}\n"
        )
    elapsed = time.monotonic() - lane_started
    if failures:
        sys.stderr.write(
            f"FAIL verify-perf: {failures} measurement(s) failed in "
            f"{elapsed:.2f}s ({POLICY_CITATION})\n"
        )
        return 1
    if elapsed > LANE_BUDGET_SECONDS:
        sys.stderr.write(
            f"FAIL verify-perf: took {elapsed:.2f}s over the "
            f"{LANE_BUDGET_SECONDS:.0f}s lane budget ({POLICY_CITATION})\n"
        )
        return 1
    sys.stdout.write(f"OK verify-perf: all measurements within their ceilings in {elapsed:.2f}s\n")
    return 0


def main(argv: Sequence[str]) -> int:
    args = list(argv[1:])
    if not args:
        return run_perf_lane()
    if args in (["--help"], ["-h"]):
        sys.stdout.write(help_text() + "\n")
        return 0
    if args == ["--metadata"]:
        sys.stdout.write(metadata_text() + "\n")
        return 0
    if args == ["--measure-player-startup"]:
        return measure_player_startup_command()
    if args == ["--measure-audio-callback-headroom"]:
        return measure_audio_callback_headroom_command()
    sys.stderr.write(help_text() + "\n")
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
