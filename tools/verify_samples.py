from __future__ import annotations

import contextlib
import json
import os
import queue
import re
import subprocess
import sys
import tempfile
import threading
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
    "collect_all.json",
    "retry_clean.json",
    "scene_edit_restore.json",
    "play_mode_edit_restore.json",
    "gold_window.json",
    "gold_collect.json",
    "combo_multiplier.json",
    "dash_cooldown.json",
    "pause_holds_simulation.json",
    "slowmo_finish.json",
    "restart_recovery.scenario.json",
    "combo_timing_sweep.json",
    "pointer_ui.scenario.json",
)
SCRIPTING_PROJECT = REPO_ROOT / "bridge" / "Unitas.Scripting"
RUNTIME_LIBRARY = REPO_ROOT / "target" / "debug" / "libunitas_runtime.dylib"
PLAYER_BINARY = REPO_ROOT / "target" / "debug" / "unitas-player"
REPORT_DIR = REPO_ROOT / "target" / "samples-lane"
PACKAGED_DIR = REPORT_DIR / "packaged"
BUILD_BUDGET_SECONDS = 600.0
RUN_BUDGET_SECONDS = 300.0
CONTROL_STDIO_CHUNK_FRAMES = 30
CONTROL_STDIO_MAX_FRAMES = 300
CONTROL_STDIO_STOP_TIMEOUT_SECONDS = 5.0
CONTROL_STATE_FIELDS = ("frames", "script_host")
# The sample game's own progress marker. The engine reports no game state, so
# the lane reads the game's log output and interprets it here.
GAME_SCORE_PATTERN = re.compile(r"Pickup collected score (\d+)")


class PackagedProofError(Exception):
    pass


SCENARIOS: tuple[tuple[str, str, tuple[str, ...], bool], ...] = (
    ("bumper_restitution", "bumper_restitution.json", ("--repeat", "2"), False),
    ("ccd_containment", "ccd_containment.json", ("--repeat", "2"), False),
    ("collision_edge", "collision_edge.json", ("--repeat", "2"), False),
    ("trigger_without_rigidbody", "trigger_without_rigidbody.json", ("--repeat", "2"), False),
    ("collision_payload", "collision_payload.json", ("--repeat", "2"), False),
    (
        "mesh_and_compound_colliders",
        "mesh_and_compound_colliders.json",
        ("--repeat", "2"),
        False,
    ),
    ("collect_all", "collect_all.json", ("--repeat", "2"), False),
    ("collect_chime", "collect_chime.json", ("--repeat", "2"), False),
    ("pause_holds_simulation", "pause_holds_simulation.json", ("--repeat", "2"), False),
    ("slowmo_finish", "slowmo_finish.json", ("--repeat", "2"), False),
    ("destroy_semantics", "destroy_semantics.json", ("--repeat", "2"), False),
    ("retry_clean", "retry_clean.json", ("--repeat", "2"), False),
    ("restart_recovery", "restart_recovery.scenario.json", ("--repeat", "2"), False),
    ("combo_multiplier", "combo_multiplier.json", ("--repeat", "2"), False),
    ("combo_timing_sweep", "combo_timing_sweep.json", ("--repeat", "2"), False),
    ("gold_window", "gold_window.json", ("--repeat", "2"), False),
    ("gold_collect", "gold_collect.json", ("--repeat", "2"), False),
    ("dash_cooldown", "dash_cooldown.json", ("--repeat", "2"), False),
    ("gamepad_drive", "gamepad_drive.scenario.json", ("--repeat", "2"), False),
    (
        "frame_independence",
        "frame_independence.json",
        ("--frame-rates", "30,60,144,240", "--repeat", "2"),
        False,
    ),
    ("pointer_ui", "pointer_ui.scenario.json", ("--repeat", "2"), False),
    ("persist_write", "persist_write.json", (), True),
    ("persist_read", "persist_read.json", (), True),
    ("persist_state_write", "persist_state_write.json", (), True),
    ("persist_state_read", "persist_state_read.json", (), True),
    ("ghost_replay", "ghost_replay.scenario.json", ("--repeat", "2"), True),
    ("s8_title_checkpoint", "s8_title_checkpoint.scenario.json", (), False),
    ("s8_arena_countdown", "s8_arena_countdown.scenario.json", (), False),
    ("s8_arena_x5_checkpoint", "s8_arena_x5_checkpoint.scenario.json", (), False),
    ("s8_results_gold_checkpoint", "s8_results_gold_checkpoint.scenario.json", (), False),
)

LANE_METADATA: tuple[tuple[str, str], ...] = (
    ("owner", "editor and engine proof"),
    ("trigger", "changes to bridge script compilation/play pipeline or samples/"),
    ("lane", "make verify-samples"),
    (
        "evidence",
        "per-scenario report JSONs, the packaged-player build report and replay "
        "verdict, and lane exit status",
    ),
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


def read_report(report_path: Path) -> dict[str, object] | None:
    report_text = report_path.read_text(encoding="utf-8")
    decoder = json.JSONDecoder()
    report: dict[str, object] | None = None
    for match in re.finditer(r"^\{", report_text, re.MULTILINE):
        try:
            parsed, _ = decoder.raw_decode(report_text[match.start() :])
        except json.JSONDecodeError:
            continue
        if isinstance(parsed, dict) and "status" in parsed:
            report = parsed
    return report


def report_messages(report: dict[str, object]) -> list[str]:
    messages = report.get("messages")
    if not isinstance(messages, list):
        return []
    texts: list[str] = []
    for entry in messages:
        if isinstance(entry, dict):
            message = entry.get("message")
            if isinstance(message, str):
                texts.append(message)
    return texts


def scenario_absent_logs(scenario_path: Path) -> tuple[str, ...]:
    document = json.loads(scenario_path.read_text(encoding="utf-8"))
    absent = document.get("absent_logs", [])
    if not isinstance(absent, list):
        return ()
    return tuple(item for item in absent if isinstance(item, str))


def check_absent_logs(scenario_path: Path, report_path: Path) -> int:
    forbidden = scenario_absent_logs(scenario_path)
    if not forbidden:
        return 0
    report = read_report(report_path)
    if report is None:
        emit(f"{scenario_path.stem}: the report carries no result to check for absent logs")
        return 1
    for message in report_messages(report):
        for literal in forbidden:
            if literal in message:
                emit(f"{scenario_path.stem}: the run produced the absent log line: {message}")
                return 1
    return 0


def check_report(report_path: Path, captured_path: Path | None = None) -> int:
    if not report_path.is_file():
        emit(f"scenario report missing at {report_path}")
        return 1
    report = read_report(report_path)
    report_status = report.get("status") if report is not None else None
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
    status = check_report(report_path, captured_path)
    if status != 0:
        return status
    return check_absent_logs(SCENARIO_DIR / scenario_file, report_path)


def native_scripted_launch(env: dict[str, str]) -> int:
    name = "native-scripted-launch"
    command = [
        str(REPO_ROOT / "target" / "debug" / "unitas-player"),
        "--sample",
        "--frames",
        "120",
    ]
    emit(f"step {name}: {' '.join(command)}")
    started = time.monotonic()
    try:
        completed = subprocess.run(
            command,
            cwd=REPO_ROOT,
            env=env,
            timeout=RUN_BUDGET_SECONDS,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
    except subprocess.TimeoutExpired:
        emit(f"step {name}: exceeded {RUN_BUDGET_SECONDS:.0f}s budget")
        return 1
    elapsed = time.monotonic() - started
    output = completed.stdout or ""
    emit(output)
    emit(f"step {name}: exit {completed.returncode} in {elapsed:.1f}s")
    if completed.returncode != 0:
        return completed.returncode
    if "script_host=coreclr" not in output:
        emit(f"step {name}: the player did not report script_host=coreclr")
        return 1
    if "frames_stepped=120" not in output:
        emit(f"step {name}: the player did not report frames_stepped=120")
        return 1
    return 0


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
        raise PackagedProofError(message)
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        message = "the packaged-player replay exceeded its time budget"
        raise PackagedProofError(message)
    try:
        process.stdin.write(json.dumps(command) + "\n")
        process.stdin.flush()
        line = responses.get(timeout=remaining)
    except OSError as error:
        message = f"control-stdio write failed: {error}"
        raise PackagedProofError(message) from error
    except queue.Empty as error:
        message = "control-stdio timed out without a response"
        raise PackagedProofError(message) from error
    if line is None:
        message = "control-stdio closed the connection without a response"
        raise PackagedProofError(message)
    try:
        payload = json.loads(line)
    except ValueError as error:
        message = f"control-stdio returned invalid JSON: {error}"
        raise PackagedProofError(message) from error
    if not isinstance(payload, dict):
        message = "control-stdio returned a non-object response"
        raise PackagedProofError(message)
    if payload.get("ok") is not True:
        detail = payload.get("error", "unknown error")
        message = f"control-stdio command {command['cmd']} failed: {detail}"
        raise PackagedProofError(message)
    return payload


def control_state(payload: dict[str, object]) -> int:
    missing = [field for field in CONTROL_STATE_FIELDS if field not in payload]
    if missing:
        names = ", ".join(missing)
        message = f"control-stdio state response lacks fields: {names}"
        raise PackagedProofError(message)
    script_host = payload["script_host"]
    if script_host != "coreclr":
        message = (
            f"control-stdio state response reports script_host={script_host!r}; "
            "the packaged player did not host the game's scripts on CoreCLR"
        )
        raise PackagedProofError(message)
    frames = payload["frames"]
    if not isinstance(frames, int) or isinstance(frames, bool):
        message = "control-stdio state response frames is not an integer"
        raise PackagedProofError(message)
    return frames


def game_score(lines: Sequence[str]) -> int | None:
    """Read the sample game's own progress marker from the player's log output."""
    score: int | None = None
    for line in lines:
        match = GAME_SCORE_PATTERN.search(line)
        if match is None:
            continue
        value = int(match.group(1))
        if score is None or value > score:
            score = value
    return score


def drive_packaged_replay(
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
                {"kind": "key", "key": ord("w"), "pressed": True},
                {"kind": "key", "key": ord("d"), "pressed": True},
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
        score = game_score(errors)
        if score is not None:
            return score
    message = (
        f"replayed input for {frames} frames but the packaged player logged no "
        "collected pickup"
    )
    raise PackagedProofError(message)


def packaged_player_replay(executable: Path, env: dict[str, str]) -> int:
    name = "packaged-player-replay"
    emit(f"step {name}: {executable} --control-stdio")
    started = time.monotonic()
    deadline = started + RUN_BUDGET_SECONDS
    responses: queue.Queue[str | None] = queue.Queue()
    process = subprocess.Popen(
        [str(executable), "--control-stdio"],
        cwd=REPO_ROOT,
        env=env,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    threading.Thread(
        target=pump_control_responses,
        args=(process, responses),
        daemon=True,
    ).start()
    errors: list[str] = []
    threading.Thread(
        target=pump_control_errors,
        args=(process, errors),
        daemon=True,
    ).start()
    try:
        score = drive_packaged_replay(process, responses, deadline, errors)
    except RuntimeError as error:
        emit(f"step {name}: {error}")
        stop_control_process(process)
        return 1
    stop_control_process(process)
    elapsed = time.monotonic() - started
    emit(f"step {name}: collected a pickup (score={score}) in {elapsed:.1f}s")
    return 0


PACKAGE_TARGETS: tuple[str, ...] = ("macos", "windows", "linux")
LAUNCHED_PACKAGE_TARGET: str = "macos"


def package_output_lines(lines: list[str], key: str, target: str) -> list[str]:
    prefix = f"{key}[{target}]="
    return [line for line in lines if line.startswith(prefix)]


def check_package_report_identity(target: str, report: dict[str, object]) -> None:
    identifier = report["build_identifier"]
    if not isinstance(identifier, str) or not identifier:
        message = f"the {target} build report has no build identifier"
        raise PackagedProofError(message)
    report_target = report["target"]
    if not isinstance(report_target, str) or not report_target.startswith(f"{target}-"):
        message = f"the {target} build report names target {report_target!r}"
        raise PackagedProofError(message)
    scenes = report["included_scenes"]
    if not isinstance(scenes, list) or not scenes:
        message = f"the {target} build report names no scenes"
        raise PackagedProofError(message)
    assets = report["assets"]
    if not isinstance(assets, list) or not assets:
        message = f"the {target} build report names no assets"
        raise PackagedProofError(message)


def check_package_report_sizes(target: str, report: dict[str, object]) -> None:
    total = report["total_size_bytes"]
    if not isinstance(total, int) or isinstance(total, bool) or total <= 0:
        message = f"the {target} build report carries no total size"
        raise PackagedProofError(message)
    contributors = report["major_contributors"]
    if not isinstance(contributors, list) or not contributors:
        message = f"the {target} build report carries no per-item sizes"
        raise PackagedProofError(message)
    for contributor in contributors:
        if not isinstance(contributor, dict):
            message = f"the {target} build report carries a malformed size entry"
            raise PackagedProofError(message)
        size = contributor.get("size_bytes")
        if not isinstance(size, int) or isinstance(size, bool):
            message = f"the {target} build report carries a malformed size entry"
            raise PackagedProofError(message)


def check_package_report_fields(target: str, report: dict[str, object]) -> None:
    required = (
        "build_identifier",
        "target",
        "included_scenes",
        "assets",
        "total_size_bytes",
        "major_contributors",
    )
    missing = [field for field in required if field not in report]
    if missing:
        names = ", ".join(missing)
        message = f"the {target} build report lacks fields: {names}"
        raise PackagedProofError(message)
    check_package_report_identity(target, report)
    check_package_report_sizes(target, report)


def check_package_not_launched(target: str, report: dict[str, object]) -> None:
    startup = report.get("startup_time")
    if not isinstance(startup, dict) or startup.get("status") != "unavailable":
        message = f"the {target} build report claims a launch this host never performed"
        raise PackagedProofError(message)
    warnings = report.get("warnings")
    if not isinstance(warnings, list):
        message = f"the {target} build report carries no warnings list"
        raise PackagedProofError(message)
    codes = [warning.get("code") for warning in warnings if isinstance(warning, dict)]
    if "package.not_launched" not in codes:
        message = f"the {target} build report does not record that it was not launched"
        raise PackagedProofError(message)


def check_packaged_report(log_path: Path) -> Path:
    if not log_path.is_file():
        emit(f"packaging log missing at {log_path}")
        message = "packaging log missing"
        raise PackagedProofError(message)
    lines = log_path.read_text(encoding="utf-8").splitlines()
    launch_artifacts: dict[str, Path] = {}
    for target in PACKAGE_TARGETS:
        packaged = package_output_lines(lines, "packaged_app", target)
        reports = package_output_lines(lines, "build_report", target)
        launchers = package_output_lines(lines, "launch_artifact", target)
        if len(packaged) != 1 or len(reports) != 1 or len(launchers) != 1:
            message = (
                "packaging output lacks one packaged_app/build_report/launch_artifact "
                f"set for {target}"
            )
            raise PackagedProofError(message)
        app_path = Path(packaged[0].removeprefix(f"packaged_app[{target}]="))
        if not app_path.is_dir():
            emit(f"packaged artifact missing at {app_path}")
            message = "packaged artifact missing"
            raise PackagedProofError(message)
        report_path = Path(reports[0].removeprefix(f"build_report[{target}]="))
        if not report_path.is_file():
            emit(f"build report missing at {report_path}")
            message = "build report missing"
            raise PackagedProofError(message)
        try:
            report = json.loads(report_path.read_text(encoding="utf-8"))
        except ValueError as error:
            emit(f"build report at {report_path} does not parse: {error}")
            message = "build report does not parse"
            raise PackagedProofError(message) from error
        check_package_report_fields(target, report)
        findings = report.get("compatibility_findings")
        if not isinstance(findings, list) or not findings:
            emit("build report has no compatibility findings")
            message = "empty compatibility findings"
            raise PackagedProofError(message)
        licensing = report.get("licensing")
        if not isinstance(licensing, dict) or not licensing.get("governing_license"):
            emit("build report has no license record")
            message = "missing license record"
            raise PackagedProofError(message)
        if target == LAUNCHED_PACKAGE_TARGET:
            emit(
                f"packaged artifact: {app_path}; build report: {report_path} "
                f"({len(findings)} compatibility findings, license "
                f"{licensing['governing_license']})"
            )
        else:
            check_package_not_launched(target, report)
            emit(
                f"packaged artifact: {app_path}; build report: {report_path} "
                f"(built on this host, not launched here: {target} players do not run "
                f"on this machine)"
            )
        launch_artifacts[target] = Path(launchers[0].removeprefix(f"launch_artifact[{target}]="))
    return launch_artifacts[LAUNCHED_PACKAGE_TARGET]


def packaged_player_proof(env: dict[str, str]) -> int:
    name = "packaged-player-proof"
    rc = run_step(
        (
            "package-sample",
            [str(PLAYER_BINARY), "--package-sample", str(PACKAGED_DIR)],
            BUILD_BUDGET_SECONDS,
            REPORT_DIR / "package-sample.log",
            REPO_ROOT,
        ),
        env,
    )
    if rc != 0:
        emit(f"step {name}: packaging failed (exit {rc})")
        return rc
    try:
        executable = check_packaged_report(REPORT_DIR / "package-sample.log")
    except PackagedProofError:
        return 1
    if packaged_player_replay(executable, env) != 0:
        return 1
    emit(f"step {name}: passed")
    return 0


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
        (
            "player-build",
            ["cargo", "build", "-p", "unitas-build", "--bin", "unitas-player"],
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
    own_persist_scenarios = frozenset({"ghost_replay"})

    results: list[tuple[str, int]] = []
    for name, scenario_file, extra_args, needs_persist in SCENARIOS:
        if not needs_persist:
            scenario_env = env
        elif name in own_persist_scenarios:
            scenario_env = dict(env)
            scenario_env["UNITAS_PERSISTENT_DATA_PATH"] = str(
                Path(tempfile.mkdtemp(prefix="prefs-", dir=REPO_ROOT / "target" / "samples-lane"))
            )
        else:
            scenario_env = persist_env
        rc = run_scenario(name, scenario_file, extra_args, scenario_env)
        results.append((name, rc))
    results.extend((step[0], run_step(step, env)) for step in editor_scenario_steps())
    results.append(("native-scripted-launch", native_scripted_launch(env)))
    results.append(("packaged-player-proof", packaged_player_proof(env)))

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
