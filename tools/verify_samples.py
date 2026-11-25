from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence

REPO_ROOT = Path(__file__).resolve().parent.parent
SAMPLE_ROOT = REPO_ROOT / "samples" / "roll-a-ball"
SCENARIO_PATH = SAMPLE_ROOT / "Scenarios" / "collect_all.json"
SCRIPTING_PROJECT = REPO_ROOT / "bridge" / "Unitas.Scripting"
RUNTIME_LIBRARY = REPO_ROOT / "target" / "debug" / "libunitas_runtime.dylib"
REPORT_PATH = REPO_ROOT / "target" / "samples-lane" / "collect_all_report.json"
BUILD_BUDGET_SECONDS = 600.0
RUN_BUDGET_SECONDS = 300.0

LANE_METADATA: tuple[tuple[str, str], ...] = (
    ("owner", "wt-01 scripting"),
    ("trigger", "changes to bridge script compilation/play pipeline or samples/"),
    ("lane", "make verify-samples"),
    ("evidence", "scenario report JSON and lane exit status"),
    ("review", "when a second sample game gains real scripts"),
)


def emit(text: str) -> None:
    sys.stdout.write(text + "\n")
    sys.stdout.flush()


def metadata_text() -> str:
    return "\n".join(f"{key}: {value}" for key, value in LANE_METADATA)


def help_text() -> str:
    usage = "usage: verify_samples.py [--help] [--metadata]"
    return f"{usage}\nlane metadata:\n{metadata_text()}"


def run_step(
    step: tuple[str, list[str], float, Path | None, Path],
    env: dict[str, str],
) -> int:
    name, command, budget, capture_path, cwd = step
    started = time.monotonic()
    emit(f"step {name}: {' '.join(command)}")
    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            timeout=budget,
            check=False,
            stdout=subprocess.PIPE if capture_path is not None else None,
            stderr=subprocess.STDOUT if capture_path is not None else None,
            text=capture_path is not None,
        )
    except subprocess.TimeoutExpired:
        emit(f"step {name}: exceeded {budget:.0f}s budget")
        return 1
    elapsed = time.monotonic() - started
    if capture_path is not None:
        output = completed.stdout or ""
        emit(output)
        capture_path.parent.mkdir(parents=True, exist_ok=True)
        capture_path.write_text(output, encoding="utf-8")
    emit(f"step {name}: exit {completed.returncode} in {elapsed:.1f}s")
    return completed.returncode


def check_report() -> int:
    if not REPORT_PATH.is_file():
        emit(f"scenario report missing at {REPORT_PATH}")
        return 1
    report_status = None
    report_text = REPORT_PATH.read_text(encoding="utf-8")
    start = report_text.find("{")
    if start >= 0:
        try:
            parsed = json.loads(report_text[start:])
        except json.JSONDecodeError:
            parsed = None
        if isinstance(parsed, dict):
            report_status = parsed.get("status")
    emit(f"scenario report status: {report_status}")
    if report_status != "passed":
        emit("verify-samples failed: scenario status is not passed")
        return 1
    emit("verify-samples passed")
    return 0


def run_lane() -> int:
    emit(metadata_text())
    if not SCENARIO_PATH.is_file():
        emit(f"scenario document missing at {SCENARIO_PATH}")
        return 1
    env = dict(os.environ)
    env["UNITAS_RUNTIME_LIBRARY"] = str(RUNTIME_LIBRARY)
    steps: list[tuple[str, list[str], float, Path | None, Path]] = [
        (
            "runtime-build",
            ["cargo", "build", "-p", "unitas-runtime"],
            BUILD_BUDGET_SECONDS,
            None,
            REPO_ROOT,
        ),
        (
            "bridge-build",
            ["dotnet", "build", "bridge/Unitas.sln", "--configuration", "Release"],
            BUILD_BUDGET_SECONDS,
            None,
            REPO_ROOT,
        ),
        (
            "scenario",
            [
                "dotnet",
                "run",
                "--project",
                str(SCRIPTING_PROJECT),
                "--configuration",
                "Release",
                "--no-build",
                "--",
                "scenario",
                str(SAMPLE_ROOT),
                str(SCENARIO_PATH),
                "--repeat",
                "2",
            ],
            RUN_BUDGET_SECONDS,
            REPORT_PATH,
            SAMPLE_ROOT,
        ),
    ]
    for step in steps:
        rc = run_step(step, env)
        if rc != 0:
            emit(f"verify-samples failed at step {step[0]} (exit {rc})")
            return 1
    return check_report()


def main(argv: Sequence[str]) -> int:
    args = list(argv)
    if "--help" in args:
        emit(help_text())
        return 0
    if "--metadata" in args:
        emit(metadata_text())
        return 0
    if args:
        emit(help_text())
        return 2
    return run_lane()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
