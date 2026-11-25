from __future__ import annotations

import os
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence

REPO_ROOT = Path(__file__).resolve().parent.parent
RUNTIME_LIBRARY = REPO_ROOT / "target" / "debug" / "libunitas_runtime.dylib"
POLICY_CITATION = "docs/ralph-workflow-policy/verification-policy.md § Gate lanes"
STEP_BUDGET_SECONDS = 300.0
LANE_BUDGET_SECONDS = 600.0
IMPORT_MEASUREMENT = "import_throughput"
IMPORT_REIMPORT_CEILING_MS = 8.4
SCRIPTING_CALL_COST = "scripting_call_cost"
PLAYERLOOP_FRAME_ALLOCATIONS = "playerloop_frame_allocations"
BRIDGE_TESTS = "bridge/Unitas.Bridge.Tests/bin/Release/net8.0/Unitas.Bridge.Tests"

LANE_METADATA: tuple[tuple[str, str], ...] = (
    ("owner", "wt-02-simulation"),
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
)

BYTES_MEASUREMENTS = frozenset({PLAYERLOOP_FRAME_ALLOCATIONS})
MEASUREMENT_PRECISION: dict[str, int] = {SCRIPTING_CALL_COST: 6}

MEASUREMENT_LINE = re.compile(r"measurement=(\S+) value_ms=([0-9.]+) ceiling_ms=([0-9.]+)")
MEASUREMENT_BYTES_LINE = re.compile(
    r"measurement=(\S+) value_bytes=([0-9.]+) ceiling_bytes=([0-9.]+)"
)
IMPORT_DURATION = re.compile(r"reimport ([0-9.]+)(ns|µs|ms|s)\b")
DURATION_FACTOR_MS: dict[str, float] = {"ns": 1e-6, "µs": 1e-3, "ms": 1.0, "s": 1e3}

BUILD_STEPS: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("runtime-build", ("cargo", "build", "-p", "unitas-runtime")),
    ("bridge-build", ("dotnet", "build", "bridge/Unitas.sln", "--configuration", "Release")),
)


def help_text() -> str:
    metadata_lines = [f"  {key}: {value}" for key, value in LANE_METADATA]
    return "\n".join(
        [
            "usage: verify_perf.py [--help] [--metadata]",
            "Runs the six perf measurements (import throughput, scene load, physics",
            "step, animation eval, scripting call cost, PlayerLoop frame allocations)",
            "and fails hard when any value breaches its ceiling.",
            "lane metadata:",
            *metadata_lines,
        ]
    )


def metadata_text() -> str:
    return "\n".join(f"{key}: {value}" for key, value in LANE_METADATA)


def parse_measurement(name: str, output: str) -> tuple[float, float] | None:
    if name == IMPORT_MEASUREMENT:
        duration = IMPORT_DURATION.search(output)
        if duration is None:
            return None
        value_ms = float(duration.group(1)) * DURATION_FACTOR_MS[duration.group(2)]
        return (value_ms, IMPORT_REIMPORT_CEILING_MS)
    if name in BYTES_MEASUREMENTS:
        bytes_match = MEASUREMENT_BYTES_LINE.search(output)
        if bytes_match is None:
            return None
        return (float(bytes_match.group(2)), float(bytes_match.group(3)))
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


def run_perf_lane() -> int:
    sys.stdout.write(metadata_text() + "\n")
    env = dict(os.environ)
    env["UNITAS_RUNTIME_LIBRARY"] = str(RUNTIME_LIBRARY)
    dotnet_root = Path.home() / ".dotnet"
    if dotnet_root.is_dir():
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
    sys.stderr.write(help_text() + "\n")
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
