from __future__ import annotations

import hashlib
import subprocess
import sys
import time
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence

REPO_ROOT = Path(__file__).resolve().parent.parent
POLICY_CITATION = "docs/ralph-workflow-policy/verification-policy.md § Gate lanes"
SAMPLE_BUDGET_SECONDS = 300.0
TEST_COMMAND: tuple[str, ...] = (
    "cargo",
    "test",
    "-p",
    "unitas-runtime",
    "--test",
    "runtime_tests",
    "roll_a_ball",
)

LANE_METADATA: tuple[tuple[str, str], ...] = (
    ("owner", "wt-02-simulation"),
    ("trigger", "every milestone touching sample content or the runtime tick"),
    ("lane", "make verify-samples"),
    ("evidence", "per-sample pass/fail lines with the sample revision"),
    ("review", "when a sample is added to or removed from the sample matrix"),
)

SAMPLES: tuple[tuple[str, str], ...] = (
    ("RollABall", "samples/RollABall/Assets/Scenes/Main.unity"),
)


def help_text() -> str:
    return (
        "usage: verify_samples.py [--metadata]\n"
        "Runs every sample's headless scenario tests through the real runtime tick\n"
        "and prints one pass/fail line per sample with the sample revision.\n"
    )


def sample_revision(scene: Path) -> str:
    if not scene.is_file():
        return "absent"
    return hashlib.sha256(scene.read_bytes()).hexdigest()[:12]


def run_samples(root: Path = REPO_ROOT) -> int:
    started = time.monotonic()
    completed = subprocess.run(
        TEST_COMMAND,
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
    )
    elapsed = time.monotonic() - started
    output = completed.stdout + completed.stderr
    failures = 0
    for name, scene_relative in SAMPLES:
        revision = sample_revision(root / scene_relative)
        scenario = f"{name} scene scenario"
        scenario_ok = "roll_a_ball_sample_scene_runs_the_scenario_headless ... ok" in output
        suite_ok = completed.returncode == 0
        if revision == "absent":
            sys.stdout.write(
                f"FAIL verify-samples ({name}): sample scene is absent at "
                f"{scene_relative} ({POLICY_CITATION})\n"
            )
            failures += 1
            continue
        if not scenario_ok or not suite_ok:
            sys.stdout.write(
                f"FAIL verify-samples ({name}@{revision}): {scenario} did not pass "
                f"(cargo exit {completed.returncode}) ({POLICY_CITATION})\n"
            )
            failures += 1
            continue
        sys.stdout.write(f"OK verify-samples ({name}@{revision}): {scenario} passed\n")
    if failures:
        sys.stdout.write(output)
        sys.stderr.write(
            f"FAIL verify-samples: {failures} sample(s) failed in {elapsed:.2f}s "
            f"({POLICY_CITATION})\n"
        )
        return 1
    if elapsed > SAMPLE_BUDGET_SECONDS:
        sys.stderr.write(
            f"FAIL verify-samples: took {elapsed:.2f}s over the "
            f"{SAMPLE_BUDGET_SECONDS:.0f}s budget ({POLICY_CITATION})\n"
        )
        return 1
    sys.stdout.write(f"OK verify-samples: all samples passed in {elapsed:.2f}s\n")
    return 0


def main(argv: Sequence[str]) -> int:
    if len(argv) > 1 and argv[1] == "--help":
        sys.stdout.write(help_text())
        return 0
    if len(argv) > 1 and argv[1] == "--metadata":
        for key, value in LANE_METADATA:
            sys.stdout.write(f"{key}: {value}\n")
        return 0
    if len(argv) > 1:
        sys.stderr.write(help_text())
        return 2
    return run_samples()


if __name__ == "__main__":
    sys.exit(main(sys.argv))
