from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence

REPO_ROOT = Path(__file__).resolve().parent.parent
SAMPLE_ROOT = REPO_ROOT / "samples" / "roll-a-ball"
SCENARIO_DIR = SAMPLE_ROOT / "Scenarios"
EDITOR_CLI = (
    REPO_ROOT / "editor" / "Unitas.Editor" / "bin" / "Release" / "net8.0" / "Unitas.Editor.dll"
)
EDITOR_SCENARIOS = (
    "collect_all_pickups.scenario.json",
    "restart_recovery.scenario.json",
)
SCRIPTING_PROJECT = REPO_ROOT / "bridge" / "Unitas.Scripting"
RUNTIME_LIBRARY = REPO_ROOT / "target" / "debug" / "libunitas_runtime.dylib"
REPORT_DIR = REPO_ROOT / "target" / "samples-lane"
BUILD_BUDGET_SECONDS = 600.0
RUN_BUDGET_SECONDS = 300.0

SCENARIOS: tuple[tuple[str, str, tuple[str, ...], bool], ...] = (
    ("bumper_restitution", "bumper_restitution.json", ("--repeat", "2"), False),
    ("ccd_containment", "ccd_containment.json", ("--repeat", "2"), False),
    ("collect_all", "collect_all.json", ("--repeat", "2"), False),
    ("pause_holds_simulation", "pause_holds_simulation.json", ("--repeat", "2"), False),
    ("slowmo_finish", "slowmo_finish.json", ("--repeat", "2"), False),
    ("destroy_semantics", "destroy_semantics.json", ("--repeat", "2"), False),
    ("retry_clean", "retry_clean.json", ("--repeat", "2"), False),
    ("combo_multiplier", "combo_multiplier.json", ("--repeat", "2"), False),
    ("gold_window", "gold_window.json", ("--repeat", "2"), False),
    ("gold_collect", "gold_collect.json", ("--repeat", "2"), False),
    ("dash_cooldown", "dash_cooldown.json", ("--repeat", "2"), False),
    (
        "frame_independence",
        "frame_independence.json",
        ("--frame-rates", "30,60,144,240", "--repeat", "2"),
        False,
    ),
    ("persist_write", "persist_write.json", (), True),
    ("persist_read", "persist_read.json", (), True),
)

LANE_METADATA: tuple[tuple[str, str], ...] = (
    ("owner", "editor and engine proof"),
    ("trigger", "changes to bridge script compilation/play pipeline or samples/"),
    ("lane", "make verify-samples"),
    ("evidence", "per-scenario report JSONs under target/samples-lane/ and lane exit status"),
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


def editor_scenario_steps() -> list[tuple[str, list[str], float, Path | None, Path]]:
    return [
        (
            f"editor-scenario-{Path(scenario).stem}",
            [
                "dotnet",
                str(EDITOR_CLI),
                "-batchmode",
                "-nographics",
                "-quit",
                "-runScenario",
                str(SCENARIO_DIR / scenario),
            ],
            RUN_BUDGET_SECONDS,
            None,
            REPO_ROOT,
        )
        for scenario in EDITOR_SCENARIOS
    ]


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


def check_report(report_path: Path, captured_path: Path | None = None) -> int:
    if not report_path.is_file():
        emit(f"scenario report missing at {report_path}")
        return 1
    report_status = None
    report_text = report_path.read_text(encoding="utf-8")
    decoder = json.JSONDecoder()
    for match in re.finditer(r"^\{", report_text, re.MULTILINE):
        try:
            parsed, _ = decoder.raw_decode(report_text[match.start() :])
        except json.JSONDecodeError:
            continue
        if isinstance(parsed, dict) and "status" in parsed:
            report_status = parsed["status"]
    emit(f"scenario report status: {report_status}")
    if report_status != "passed":
        return 1
    if captured_path is not None:
        captured = captured_path.read_text(encoding="utf-8") if captured_path.is_file() else ""
        if "sweep: pass" not in captured:
            emit(f"sweep verdict missing in {captured_path}")
            return 1
    return 0


def run_scenario(
    name: str,
    scenario_file: str,
    extra_args: tuple[str, ...],
    env: dict[str, str],
) -> int:
    report_path = REPORT_DIR / f"{name}_report.json"
    command = [
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
        str(SCENARIO_DIR / scenario_file),
        *extra_args,
    ]
    rc = run_step(
        (f"scenario-{name}", command, RUN_BUDGET_SECONDS, report_path, SAMPLE_ROOT),
        env,
    )
    if rc != 0:
        return rc
    captured_path = report_path if "--frame-rates" in extra_args else None
    return check_report(report_path, captured_path)


def run_lane() -> int:
    emit(metadata_text())
    env = dict(os.environ)
    env["UNITAS_RUNTIME_LIBRARY"] = str(RUNTIME_LIBRARY)
    env.pop("UNITAS_PERSISTENT_DATA_PATH", None)

    build_steps: list[tuple[str, list[str], float, Path | None, Path]] = [
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
    ]
    for step in build_steps:
        rc = run_step(step, env)
        if rc != 0:
            emit(f"verify-samples failed at step {step[0]} (exit {rc})")
            return 1

    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    persist_dir = Path(tempfile.mkdtemp(prefix="prefs-", dir=REPO_ROOT / "target" / "samples-lane"))
    persist_env = dict(env)
    persist_env["UNITAS_PERSISTENT_DATA_PATH"] = str(persist_dir)

    results: list[tuple[str, int]] = []
    for name, scenario_file, extra_args, needs_persist in SCENARIOS:
        scenario_env = persist_env if needs_persist else env
        rc = run_scenario(name, scenario_file, extra_args, scenario_env)
        results.append((name, rc))
    results.extend((step[0], run_step(step, env)) for step in editor_scenario_steps())

    emit("scenario summary:")
    for name, rc in results:
        emit(f"  {name}: {'passed' if rc == 0 else f'FAILED (exit {rc})'}")
    failed = [name for name, rc in results if rc != 0]
    if failed:
        emit(f"verify-samples failed: {len(failed)} scenario(s) failed: {', '.join(failed)}")
        return 1
    emit("verify-samples passed")
    return 0


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
