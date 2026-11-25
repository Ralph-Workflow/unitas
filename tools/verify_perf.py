from __future__ import annotations

import re
import subprocess
import sys
import time
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence

REPO_ROOT = Path(__file__).resolve().parent.parent
POLICY_CITATION = "docs/ralph-workflow-policy/verification-policy.md § Gate lanes"
STEP_BUDGET_SECONDS = 300.0
LANE_BUDGET_SECONDS = 600.0
IMPORT_MEASUREMENT = "import_throughput"
IMPORT_REIMPORT_CEILING_MS = 8.4

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
)

MEASUREMENT_LINE = re.compile(r"measurement=(\S+) value_ms=([0-9.]+) ceiling_ms=([0-9.]+)")
IMPORT_DURATION = re.compile(r"reimport ([0-9.]+)(ns|µs|ms|s)\b")
DURATION_FACTOR_MS: dict[str, float] = {"ns": 1e-6, "µs": 1e-3, "ms": 1.0, "s": 1e3}


def help_text() -> str:
    metadata_lines = [f"  {key}: {value}" for key, value in LANE_METADATA]
    return "\n".join(
        [
            "usage: verify_perf.py [--help] [--metadata]",
            "Runs the four perf measurements (import throughput, scene load, physics",
            "step, animation eval) and fails hard when any value breaches its ceiling.",
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
    match = MEASUREMENT_LINE.search(output)
    if match is None:
        return None
    return (float(match.group(2)), float(match.group(3)))


def run_measurement(
    name: str,
    command: Sequence[str],
    budget_seconds: float,
) -> tuple[float, float] | None:
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
    lane_started = time.monotonic()
    failures = 0
    for name, command in MEASUREMENTS:
        measurement = run_measurement(name, command, STEP_BUDGET_SECONDS)
        if measurement is None:
            failures += 1
            continue
        value_ms, ceiling_ms = measurement
        if value_ms > ceiling_ms:
            sys.stderr.write(
                f"FAIL verify-perf ({name}): value_ms={value_ms:.3f} breaches "
                f"ceiling_ms={ceiling_ms:.3f} ({POLICY_CITATION})\n"
            )
            failures += 1
            continue
        sys.stdout.write(
            f"OK verify-perf ({name}): value_ms={value_ms:.3f} ceiling_ms={ceiling_ms:.3f}\n"
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
