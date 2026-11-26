from __future__ import annotations

import hashlib
import json
import os
import re
import secrets
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

from tools import verify_samples

if TYPE_CHECKING:
    from collections.abc import Sequence

REPO_ROOT = Path(__file__).resolve().parent.parent
SAMPLES_DIR = REPO_ROOT / "samples"
WORKSPACE_ROOT = REPO_ROOT / "target" / "benchmark-workspace"
WORKSPACE_SAMPLES = WORKSPACE_ROOT / "samples"
REPORT_DIR = REPO_ROOT / "target" / "benchmark-lane"
SCENARIO_REPORT_DIR = REPORT_DIR / "scenarios"
PERSIST_DIR = REPORT_DIR / "persist"
LANE_REPORT = REPORT_DIR / "report.json"
BENCHMARK_REPORT = REPORT_DIR / "benchmark-report.json"
BENCHMARK_PROJECT_DIR = REPO_ROOT / "editor" / "Unitas.Benchmark"
BENCHMARK_DLL = BENCHMARK_PROJECT_DIR / "bin" / "Release" / "net8.0" / "Unitas.Benchmark.dll"
EDITOR_DLL = (
    REPO_ROOT / "editor" / "Unitas.Editor" / "bin" / "Release" / "net8.0" / "Unitas.Editor.dll"
)
SOLUTION = REPO_ROOT / "bridge" / "Unitas.sln"
RUNTIME_LIBRARY = REPO_ROOT / "target" / "debug" / "libunitas_runtime.dylib"
DATA_RELATIVE = Path("samples/roll-a-ball/Benchmarks/agent-authoring/bonus-pickup")
DATA_DIR = REPO_ROOT / DATA_RELATIVE
WORKSPACE_DATA_DIR = WORKSPACE_ROOT / DATA_RELATIVE
REQUEST_NAME = "REQUEST.md"
PLAN_NAME = "agent-steps.json"
ACCEPTANCE_NAME = "acceptance.scenario.json"
EXPECTED_FILES_NAME = "expected-files.json"
DATA_PREFIX = "samples/roll-a-ball/Benchmarks/"
CREDENTIAL_VARIABLE = "UNITAS_PERSON_CREDENTIAL"
BUILD_BUDGET_SECONDS = 900.0
BENCHMARK_BUDGET_SECONDS = 900.0
SCENARIO_BUDGET_SECONDS = 300.0
ARTIFACT_DIRECTORIES = frozenset({"Library", "Logs", "Temp", "UserSettings", "bin", "obj"})
ASSET_PATH_PATTERN = re.compile(r"^samples/[A-Za-z0-9_.\-/]+\.[A-Za-z0-9]+$")
EXPECTED_FILE_KEYS = ("files", "paths", "expectedFiles", "expected_files")

LANE_METADATA: tuple[tuple[str, str], ...] = (
    ("owner", "editor and engine proof"),
    (
        "trigger",
        "changes to the editor's MCP tool surface, the agent-authoring benchmark, "
        "or the change-review policy",
    ),
    ("lane", "make verify-benchmark"),
    (
        "evidence",
        "the lane report at target/benchmark-lane/report.json, its per-criterion "
        "OK/FAIL lines, and the lane exit status",
    ),
    (
        "review",
        "when the agent-authoring benchmark, its acceptance scenario, or the "
        "change-review policy changes",
    ),
)


@dataclass
class Criterion:
    name: str
    status: str
    detail: str

    @property
    def ok(self) -> bool:
        return self.status == "passed"


def emit(text: str) -> None:
    sys.stdout.write(text + "\n")
    sys.stdout.flush()


def metadata_text() -> str:
    return "\n".join(f"{key}: {value}" for key, value in LANE_METADATA)


def help_text() -> str:
    usage = "usage: python3 -m tools.verify_benchmark [--help] [--metadata]"
    return f"{usage}\nlane metadata:\n{metadata_text()}"


def record(criteria: list[Criterion], name: str, *, ok: bool, detail: str) -> bool:
    criterion = Criterion(name, "passed" if ok else "failed", detail)
    criteria.append(criterion)
    emit(f"criterion {name}: {criterion.status} ({detail})")
    return criterion.ok


def finish(criteria: list[Criterion], started: float) -> int:
    failed = [criterion for criterion in criteria if not criterion.ok]
    duration = time.monotonic() - started
    document = {
        "lane": "make verify-benchmark",
        "criteria": [
            {"name": criterion.name, "ok": criterion.ok, "detail": criterion.detail}
            for criterion in criteria
        ],
        "ok": not failed,
        "durationSeconds": round(duration, 3),
    }
    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    LANE_REPORT.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    emit(f"lane report: {LANE_REPORT}")
    if failed:
        names = ", ".join(criterion.name for criterion in failed)
        emit(f"verify-benchmark failed: {names}")
        emit(
            "a red lane is a red lane: fix the failing criterion and run the lane again "
            "(docs/ralph-workflow-policy/gate-script-policy.md Failure output; "
            "docs/ralph-workflow-policy/verification-policy.md Gate lanes)"
        )
        return 1
    emit(f"verify-benchmark passed in {duration:.1f}s")
    return 0


def lane_environment() -> dict[str, str]:
    env = dict(os.environ)
    env["UNITAS_RUNTIME_LIBRARY"] = str(RUNTIME_LIBRARY)
    env.pop("UNITAS_PERSISTENT_DATA_PATH", None)
    return env


def missing_inputs() -> list[str]:
    missing: list[str] = []
    if not BENCHMARK_PROJECT_DIR.is_dir():
        missing.append("editor/Unitas.Benchmark/")
    missing.extend(
        f"{DATA_RELATIVE.as_posix()}/{name}"
        for name in (REQUEST_NAME, PLAN_NAME, ACCEPTANCE_NAME, EXPECTED_FILES_NAME)
        if not (DATA_DIR / name).is_file()
    )
    if not EDITOR_DLL.is_file():
        missing.append("editor/Unitas.Editor/bin/Release/net8.0/Unitas.Editor.dll")
    if not RUNTIME_LIBRARY.is_file():
        missing.append("target/debug/libunitas_runtime.dylib")
    return missing


def run_command(
    command: list[str],
    cwd: Path,
    env: dict[str, str],
    budget: float,
) -> tuple[int, str]:
    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            timeout=budget,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
    except subprocess.TimeoutExpired as error:
        output = error.stdout if isinstance(error.stdout, str) else ""
        return 1, f"the command exceeded its {budget:.0f}s budget\n{output}"
    except OSError as error:
        return 1, f"the command could not run: {error}"
    return completed.returncode, completed.stdout or ""


def file_digest(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(65536), b""):
            digest.update(block)
    return digest.hexdigest()


def content_snapshot(root: Path) -> dict[str, str]:
    snapshot: dict[str, str] = {}
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(root)
        if any(part in ARTIFACT_DIRECTORIES for part in relative.parts[:-1]):
            continue
        snapshot[relative.as_posix()] = file_digest(path)
    return snapshot


def changed_paths(before: dict[str, str], after: dict[str, str]) -> list[str]:
    changed = [path for path, digest in before.items() if after.get(path) != digest]
    changed.extend(path for path in after if path not in before)
    return sorted(changed)


def read_report(path: Path) -> dict[str, object] | None:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if isinstance(document, dict):
        return document
    return None


def normalize_path(value: str) -> str:
    return value.strip().replace("\\", "/").removeprefix("./")


def read_expected_paths() -> set[str] | None:
    try:
        document = json.loads((DATA_DIR / EXPECTED_FILES_NAME).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    entries: object = document
    if isinstance(document, dict):
        for key in EXPECTED_FILE_KEYS:
            if key in document:
                entries = document[key]
                break
        else:
            return None
    if not isinstance(entries, list):
        return None
    paths: set[str] = set()
    for entry in entries:
        if isinstance(entry, str):
            paths.add(normalize_path(entry))
        elif isinstance(entry, dict):
            value = entry.get("path")
            if isinstance(value, str):
                paths.add(normalize_path(value))
    return paths


def collect_asset_paths(node: object, paths: set[str]) -> None:
    if isinstance(node, str):
        normalized = normalize_path(node)
        if normalized.startswith("Assets/"):
            normalized = "samples/roll-a-ball/" + normalized
        if ASSET_PATH_PATTERN.match(normalized) and not normalized.startswith(DATA_PREFIX):
            paths.add(normalized)
        return
    if isinstance(node, dict):
        for value in node.values():
            collect_asset_paths(value, paths)
        return
    if isinstance(node, list):
        for value in node:
            collect_asset_paths(value, paths)


def change_set_paths(report: dict[str, object]) -> set[str]:
    paths: set[str] = set()
    collect_asset_paths(report.get("steps"), paths)
    collect_asset_paths(report.get("summary"), paths)
    return paths


def describe(paths: set[str]) -> str:
    return ", ".join(sorted(paths)) if paths else "no path"


def check_steps(steps: object) -> tuple[bool, str]:
    if not isinstance(steps, list) or not steps:
        return False, "the benchmark report carries no steps"
    failures: list[str] = []
    for index, step in enumerate(steps):
        if not isinstance(step, dict):
            failures.append(f"step {index} is not an object")
            continue
        if step.get("ok") is not True:
            failures.append(
                f"step {index} ({step.get('tool')}) did not succeed: {step.get('error')}"
            )
    if failures:
        return False, "; ".join(failures)
    return True, f"all {len(steps)} steps succeeded"


def check_tools(report: dict[str, object], steps: object) -> tuple[bool, str]:
    tools = report.get("tools")
    if not isinstance(tools, list) or not tools:
        return False, "the benchmark report advertises no tools"
    advertised = {name for name in tools if isinstance(name, str)}
    used = {
        step["tool"]
        for step in steps or []
        if isinstance(step, dict) and isinstance(step.get("tool"), str)
    }
    missing = sorted(used - advertised)
    if missing:
        return False, "the run called tools the editor does not advertise: " + ", ".join(missing)
    return True, f"{len(advertised)} advertised tools cover every tool the run called"


def check_expected_files(report: dict[str, object]) -> tuple[bool, str]:
    expected = read_expected_paths()
    if expected is None:
        return (
            False,
            "expected-files.json carries no path list; it must hold a JSON array of "
            "paths or an object with a files, paths, expectedFiles or expected_files array",
        )
    actual = change_set_paths(report)
    if not actual:
        return False, "the benchmark report names no change-set path to compare"
    if actual != expected:
        return (
            False,
            f"the change set touches {describe(actual)} but expected-files.json names "
            f"{describe(expected)}",
        )
    return True, "the change set touches exactly the paths expected-files.json names"


def check_rejected(steps: object) -> tuple[bool, str]:
    if isinstance(steps, list):
        for step in steps:
            if not isinstance(step, dict):
                continue
            if step.get("tool") == "changes.reject" and step.get("ok") is True:
                return True, "the report records the rejection of the change set"
    return (
        False,
        "the report records no successful changes.reject step; the plan must reject the "
        "change set as its person step",
    )


def redact(text: str, credential: str) -> str:
    if not credential:
        return text
    return text.replace(credential, "<redacted>")


def check_inputs(criteria: list[Criterion]) -> bool:
    missing = missing_inputs()
    if missing:
        return record(criteria, "inputs-present", ok=False, detail="missing " + ", ".join(missing))
    return record(
        criteria,
        "inputs-present",
        ok=True,
        detail=(
            "the benchmark project, the benchmark data, the editor assembly and the "
            "runtime library are in the tree"
        ),
    )


def build_lane(criteria: list[Criterion], env: dict[str, str]) -> bool:
    build_rc, build_output = run_command(
        ["dotnet", "build", str(SOLUTION), "--configuration", "Release"],
        REPO_ROOT,
        env,
        BUILD_BUDGET_SECONDS,
    )
    if build_rc != 0:
        emit(build_output)
        return record(
            criteria,
            "build",
            ok=False,
            detail=f"the solution build exited {build_rc}",
        )
    record(criteria, "build", ok=True, detail="the solution built in Release")
    if not BENCHMARK_DLL.is_file():
        return record(
            criteria,
            "benchmark-assembly",
            ok=False,
            detail=f"the build produced no benchmark assembly at {BENCHMARK_DLL}",
        )
    return record(
        criteria,
        "benchmark-assembly",
        ok=True,
        detail=f"the benchmark assembly is at {BENCHMARK_DLL}",
    )


def prepare_workspace(criteria: list[Criterion]) -> bool:
    if WORKSPACE_ROOT.exists():
        shutil.rmtree(WORKSPACE_ROOT)
    WORKSPACE_ROOT.mkdir(parents=True)
    shutil.copytree(
        SAMPLES_DIR,
        WORKSPACE_SAMPLES,
        ignore=shutil.ignore_patterns(*ARTIFACT_DIRECTORIES),
    )
    return record(
        criteria,
        "workspace-copied",
        ok=True,
        detail="samples/ copied into target/benchmark-workspace/samples/",
    )


def run_benchmark(criteria: list[Criterion], env: dict[str, str]) -> dict[str, object] | None:
    credential = secrets.token_urlsafe(32)
    child_env = dict(env)
    child_env[CREDENTIAL_VARIABLE] = credential
    command = [
        "dotnet",
        str(BENCHMARK_DLL),
        "--editor",
        str(EDITOR_DLL),
        "--project",
        str(WORKSPACE_ROOT),
        "--plan",
        str(WORKSPACE_DATA_DIR / PLAN_NAME),
        "--report",
        str(BENCHMARK_REPORT),
    ]
    emit("step benchmark: " + " ".join(command))
    benchmark_rc, benchmark_output = run_command(
        command,
        WORKSPACE_ROOT,
        child_env,
        BENCHMARK_BUDGET_SECONDS,
    )
    report_text = ""
    if BENCHMARK_REPORT.is_file():
        report_text = BENCHMARK_REPORT.read_text(encoding="utf-8", errors="replace")
    leaked = credential in benchmark_output or credential in report_text
    emit(redact(benchmark_output, credential))
    record(
        criteria,
        "benchmark-exit",
        ok=benchmark_rc == 0,
        detail=f"the benchmark exited {benchmark_rc}",
    )
    record(
        criteria,
        "credential-not-leaked",
        ok=not leaked,
        detail=(
            "the person credential reached neither the benchmark output nor its report"
            if not leaked
            else "the person credential reached the benchmark output or its report"
        ),
    )
    report = read_report(BENCHMARK_REPORT)
    if report is None:
        record(
            criteria,
            "benchmark-report",
            ok=False,
            detail=f"the benchmark wrote no report object at {BENCHMARK_REPORT}",
        )
        return None
    record(
        criteria,
        "benchmark-report",
        ok=True,
        detail=f"the benchmark report at {BENCHMARK_REPORT} carries steps, tools and a summary",
    )
    return report


def check_acceptance_step(steps: object) -> tuple[bool, str]:
    if isinstance(steps, list):
        for step in steps:
            if not isinstance(step, dict) or step.get("tool") != "scenario.run":
                continue
            result = step.get("result")
            content = result.get("content") if isinstance(result, dict) else None
            if isinstance(content, dict) and content.get("status") == "passed":
                return True, "the acceptance scenario passed in editor play mode"
            return False, f"the acceptance scenario did not pass: {content}"
    return False, "the report records no scenario.run step"


def check_report_criteria(criteria: list[Criterion], report: dict[str, object]) -> None:
    steps = report.get("steps")
    steps_ok, steps_detail = check_steps(steps)
    record(criteria, "steps-succeeded", ok=steps_ok, detail=steps_detail)
    tools_ok, tools_detail = check_tools(report, steps)
    record(criteria, "tools-advertised", ok=tools_ok, detail=tools_detail)
    acceptance_ok, acceptance_detail = check_acceptance_step(steps)
    record(criteria, "acceptance-scenario", ok=acceptance_ok, detail=acceptance_detail)
    files_ok, files_detail = check_expected_files(report)
    record(criteria, "expected-files", ok=files_ok, detail=files_detail)
    rejected_ok, rejected_detail = check_rejected(steps)
    record(criteria, "change-set-rejected", ok=rejected_ok, detail=rejected_detail)


def rerun_sample_scenarios(env: dict[str, str]) -> tuple[bool, str]:
    verify_samples.SAMPLE_ROOT = WORKSPACE_SAMPLES / "roll-a-ball"
    verify_samples.SCENARIO_DIR = WORKSPACE_SAMPLES / "roll-a-ball" / "Scenarios"
    verify_samples.REPORT_DIR = SCENARIO_REPORT_DIR
    persist_env = dict(env)
    persist_env["UNITAS_PERSISTENT_DATA_PATH"] = str(PERSIST_DIR)
    failures: list[str] = []
    for name, scenario_file, extra_args, needs_persist in verify_samples.SCENARIOS:
        scenario_env = persist_env if needs_persist else env
        rc = verify_samples.run_scenario(name, scenario_file, extra_args, scenario_env)
        if rc != 0:
            failures.append(f"{name} (exit {rc})")
    if failures:
        return False, "the workspace scenarios failed: " + ", ".join(failures)
    return True, f"all {len(verify_samples.SCENARIOS)} sample scenarios passed in the workspace"


def check_workspace_criteria(
    criteria: list[Criterion],
    workspace_snapshot: dict[str, str],
    repository_snapshot: dict[str, str],
) -> None:
    workspace_changes = changed_paths(workspace_snapshot, content_snapshot(WORKSPACE_SAMPLES))
    record(
        criteria,
        "workspace-unchanged",
        ok=not workspace_changes,
        detail=(
            "the workspace holds the content it started with"
            if not workspace_changes
            else "the workspace changed: " + ", ".join(workspace_changes)
        ),
    )
    repository_changes = changed_paths(repository_snapshot, content_snapshot(SAMPLES_DIR))
    record(
        criteria,
        "repository-samples-untouched",
        ok=not repository_changes,
        detail=(
            "the repository samples/ tree is untouched"
            if not repository_changes
            else "the repository samples/ tree changed: " + ", ".join(repository_changes)
        ),
    )


def run_lane() -> int:
    emit(metadata_text())
    started = time.monotonic()
    criteria: list[Criterion] = []
    env = lane_environment()
    if not check_inputs(criteria):
        return finish(criteria, started)
    if not build_lane(criteria, env):
        return finish(criteria, started)
    repository_snapshot = content_snapshot(SAMPLES_DIR)
    if not prepare_workspace(criteria):
        return finish(criteria, started)
    workspace_snapshot = content_snapshot(WORKSPACE_SAMPLES)
    report = run_benchmark(criteria, env)
    if report is None:
        return finish(criteria, started)
    check_report_criteria(criteria, report)
    scenarios_ok, scenarios_detail = rerun_sample_scenarios(env)
    record(criteria, "sample-scenarios", ok=scenarios_ok, detail=scenarios_detail)
    check_workspace_criteria(criteria, workspace_snapshot, repository_snapshot)
    return finish(criteria, started)


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
