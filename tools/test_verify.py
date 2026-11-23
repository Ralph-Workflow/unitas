from __future__ import annotations

import ast
import re
from pathlib import Path

from tools import verify


def test_repository_gates_report_no_findings() -> None:
    for name in sorted(verify.GATES):
        findings = verify.GATES[name]()
        assert findings == [], f"{name} reported findings: {findings}"


def test_placeholder_tokens_are_detected() -> None:
    corpus = {"testing-policy.md": "RALPH-FACT: test_command: REPLACE-ME"}
    findings = verify.check_placeholders(corpus)
    assert findings == ["testing-policy.md: placeholder token 'REPLACE-ME' remains"]


def test_duplicate_fact_keys_are_detected() -> None:
    corpus = {
        "testing-policy.md": "RALPH-FACT: test_command: a\nRALPH-FACT: test_command: b\n",
    }
    findings = verify.check_fact_keys(corpus)
    assert findings == ["testing-policy.md: RALPH-FACT key 'test_command' occurs 2 times"]


def test_unapproved_command_tool_is_detected() -> None:
    corpus = {"testing-policy.md": "RALPH-COMMAND: ./run-tests.sh\n"}
    findings = verify.check_commands(corpus)
    assert findings == ["testing-policy.md:1: RALPH-COMMAND tool './run-tests.sh' is not approved"]


def test_unknown_guard_enforcer_is_detected() -> None:
    corpus = {"testing-policy.md": _guard_table("deterministic: make nowhere")}
    findings = verify.check_guard_enforcement(corpus, ["make verify"])
    assert findings == ["testing-policy.md: guard row 1 references unknown enforcer 'make nowhere'"]


def test_advisory_guard_enforcement_is_accepted() -> None:
    corpus = {
        "testing-policy.md": _guard_table("advisory: no gate exists; review: when a gate lands")
    }
    assert verify.check_guard_enforcement(corpus, []) == []


def test_repository_has_no_bypass_markers() -> None:
    assert verify.check_bypass() == []


def test_bypass_marker_in_source_is_detected(tmp_path: Path) -> None:
    source = tmp_path / "sample.py"
    source.write_text("value = 1  # noqa: E501\n", encoding="utf-8")
    findings = verify.bypass_findings_in(source)
    assert any("noqa" in finding for finding in findings)


def test_banned_construct_is_detected() -> None:
    tree = ast.parse("def run() -> None:\n    eval('1')\n")
    findings = verify.banned_findings("sample.py", tree)
    assert any("banned call eval" in finding for finding in findings)


def test_dependency_gate_allows_stdlib_only() -> None:
    assert verify.check_dependencies() == []


def test_secret_delta_reports_new_findings() -> None:
    baseline = {"a.py": [{"type": "Hex High Entropy String", "line_number": 3}]}
    scanned = {
        "a.py": [{"type": "Hex High Entropy String", "line_number": 3}],
        "b.py": [{"type": "Keyword Detector", "line_number": 7}],
    }
    assert verify.secret_delta(baseline, scanned) == [
        "secret finding not in baseline: b.py Keyword Detector"
    ]


def test_secret_scan_excludes_build_output_but_not_sources() -> None:
    build_output = (
        "target/CACHEDIR.TAG",
        "crates/unitas-asset/target/debug/deps/libunitas_asset.rlib",
    )
    for path in build_output:
        assert any(re.search(pattern, path) for pattern in verify.SECRET_SCAN_EXCLUDES)
    source = "crates/unitas-core/src/scene_graph.rs"
    assert not any(re.search(pattern, source) for pattern in verify.SECRET_SCAN_EXCLUDES)


def _guard_table(enforcement: str) -> str:
    header = (
        "| Guard | Owner | Protected outcome | Detected condition | Claim scope | "
        "Enforcement | Retirement trigger |"
    )
    separator = "|---|---|---|---|---|---|---|"
    row = f"| G | O | p | d | c | {enforcement} | r |"
    return "## Guard inventory\n\n" + header + "\n" + separator + "\n" + row + "\n"
